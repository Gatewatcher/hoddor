use super::converters;
use crate::adapters::wasm::SimpleGraphAdapter;
use crate::platform::Platform;
use crate::ports::graph::GraphPort;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// Global singleton graph adapter - shared across all WASM function calls
static GRAPH: Lazy<SimpleGraphAdapter> = Lazy::new(|| SimpleGraphAdapter::new());

// Global platform for dependency injection (provides storage, encryption, etc.)
static PLATFORM: Lazy<Platform> = Lazy::new(|| Platform::new());

#[derive(Serialize, Deserialize)]
pub struct GraphNodeResult {
    pub id: String,
    pub node_type: String,
    pub encrypted_content: Vec<u8>,
    pub content_hmac: String,
    pub labels: Vec<String>,
    pub similarity: Option<f32>,
}

/// Create a memory node in the graph with encrypted content and embedding
/// Note: Content should be encrypted by the caller using Age encryption
#[wasm_bindgen]
pub async fn graph_create_memory_node(
    vault_name: &str,
    encrypted_content: Vec<u8>,
    content_hmac: String,
    embedding: Vec<f32>,
    labels: Vec<String>,
) -> Result<String, JsValue> {
    // Get vault ID from vault name (use vault_name as ID for now)
    let vault_id = vault_name;

    // Use global singleton graph (shared across all calls)
    let node_id = GRAPH
        .create_node(
            vault_id,
            "memory",
            encrypted_content,
            content_hmac,
            labels,
            Some(embedding),
            Some("user_memories".to_string()),
        )
        .await
        .map_err(converters::to_js_error)?;

    Ok(node_id.as_str().to_string())
}

/// Search for similar nodes using vector similarity
/// Returns encrypted content - caller must decrypt
#[wasm_bindgen]
pub async fn graph_vector_search(
    vault_name: &str,
    query_embedding: Vec<f32>,
    limit: usize,
    min_similarity: Option<f32>,
) -> Result<JsValue, JsValue> {
    let vault_id = vault_name;

    // Use global singleton graph (shared across all calls)
    let results = GRAPH
        .vector_search(vault_id, query_embedding, limit, min_similarity)
        .await
        .map_err(converters::to_js_error)?;

    // Convert to JS-friendly format (content remains encrypted)
    let js_results: Vec<GraphNodeResult> = results
        .into_iter()
        .map(|(node, similarity)| GraphNodeResult {
            id: node.id.as_str().to_string(),
            node_type: node.node_type,
            encrypted_content: node.encrypted_content,
            content_hmac: node.content_hmac,
            labels: node.labels,
            similarity: Some(similarity),
        })
        .collect();

    serde_wasm_bindgen::to_value(&js_results).map_err(converters::to_js_error)
}

/// Get a specific node by ID
/// Returns encrypted content - caller must decrypt
#[wasm_bindgen]
pub async fn graph_get_node(_vault_name: &str, _node_id: &str) -> Result<JsValue, JsValue> {
    // Use global singleton graph when implemented
    // let results = GRAPH.get_node(...).await?;

    // For MVP, we'll implement this later when we need it
    Err(JsValue::from_str(
        "graph_get_node not yet implemented - use vector_search instead",
    ))
}

/// List all memory nodes for a vault
/// Returns encrypted content - caller must decrypt
#[wasm_bindgen]
pub async fn graph_list_memory_nodes(
    vault_name: &str,
    limit: Option<usize>,
) -> Result<JsValue, JsValue> {
    let vault_id = vault_name;

    // Use global singleton graph
    let nodes = GRAPH
        .list_nodes_by_type(vault_id, "memory", limit)
        .await
        .map_err(converters::to_js_error)?;

    // Convert to JS-friendly format (content remains encrypted)
    let js_results: Vec<GraphNodeResult> = nodes
        .into_iter()
        .map(|node| GraphNodeResult {
            id: node.id.as_str().to_string(),
            node_type: node.node_type,
            encrypted_content: node.encrypted_content,
            content_hmac: node.content_hmac,
            labels: node.labels,
            similarity: None,
        })
        .collect();

    serde_wasm_bindgen::to_value(&js_results).map_err(converters::to_js_error)
}

/// Save the graph for a vault to OPFS with Age encryption
///
/// # Arguments
/// * `vault_name` - The name of the vault
/// * `recipient` - Age public key (from identity.public_key)
/// * `identity` - Age private key (from identity.private_key)
///
/// # Note
/// This is a pragmatic implementation that duplicates GraphPersistence logic
/// TODO: Refactor to use GraphPersistence with Arc<Mutex<>> or references
#[wasm_bindgen]
pub async fn graph_backup_vault(
    vault_name: &str,
    recipient: &str,
    _identity: &str,
) -> Result<(), JsValue> {
    use crate::domain::crypto;
    use crate::adapters::wasm::graph_persistence::GraphBackup;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    // 1. Export all nodes from the vault
    let mut all_nodes = Vec::new();
    let node_types = vec!["memory", "entity", "event", "concept", "conversation", "document", "preference"];

    for node_type in node_types {
        match GRAPH.list_nodes_by_type(vault_name, node_type, None).await {
            Ok(nodes) => all_nodes.extend(nodes),
            Err(_) => continue,
        }
    }

    // 2. Export all edges (iterate over nodes to get their edges)
    let mut all_edges = Vec::new();
    let mut seen_edge_ids = std::collections::HashSet::new();

    for node in &all_nodes {
        match GRAPH.get_edges(vault_name, &node.id, crate::domain::graph::EdgeDirection::Both).await {
            Ok(edges) => {
                for edge in edges {
                    if seen_edge_ids.insert(edge.id.clone()) {
                        all_edges.push(edge);
                    }
                }
            }
            Err(_) => continue,
        }
    }

    // 3. Create backup structure
    let backup = GraphBackup {
        version: 1,
        nodes: all_nodes,
        edges: all_edges,
        created_at: js_sys::Date::now() as u64,
    };

    // 4. Serialize to JSON
    let json = serde_json::to_string(&backup)
        .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))?;

    // 5. Encrypt with Age
    let encrypted = crypto::encrypt_for_recipients(
        &*PLATFORM,
        json.as_bytes(),
        &[recipient],
    )
    .await
    .map_err(|e| JsValue::from_str(&format!("Encryption failed: {}", e)))?;

    // 6. Encode to base64
    let data_to_save = BASE64.encode(&encrypted);

    // 7. Create directory if needed
    PLATFORM.storage()
        .create_directory("graph_backups")
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create directory: {}", e)))?;

    // 8. Save to OPFS
    let file_path = format!("graph_backups/{}.age", vault_name);
    PLATFORM.storage()
        .write_file(&file_path, &data_to_save)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to write file: {}", e)))?;

    Ok(())
}

/// Restore the graph for a vault from OPFS
///
/// # Arguments
/// * `vault_name` - The name of the vault
/// * `recipient` - Age public key (not used for decryption)
/// * `identity` - Age private key (for decryption)
///
/// # Returns
/// * `true` if backup was found and restored
/// * `false` if no backup exists (first time)
///
/// # Note
/// This is a pragmatic implementation that duplicates GraphPersistence logic
/// TODO: Refactor to use GraphPersistence with Arc<Mutex<>> or references
#[wasm_bindgen]
pub async fn graph_restore_vault(
    vault_name: &str,
    _recipient: &str,
    identity: &str,
) -> Result<bool, JsValue> {
    use crate::domain::crypto;
    use crate::adapters::wasm::graph_persistence::GraphBackup;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    // 1. Check if backup exists
    let file_path = format!("graph_backups/{}.age", vault_name);
    let file_content = match PLATFORM.storage().read_file(&file_path).await {
        Ok(content) => content,
        Err(_) => return Ok(false), // No backup found
    };

    // 2. Decode from base64
    let encrypted = BASE64.decode(&file_content)
        .map_err(|e| JsValue::from_str(&format!("Base64 decode failed: {}", e)))?;

    // 3. Decrypt with Age
    let decrypted = crypto::decrypt_with_identity(
        &*PLATFORM,
        &encrypted,
        identity,
    )
    .await
    .map_err(|e| JsValue::from_str(&format!("Decryption failed: {}", e)))?;

    // 4. Convert to String
    let json = String::from_utf8(decrypted)
        .map_err(|e| JsValue::from_str(&format!("UTF-8 conversion failed: {}", e)))?;

    // 5. Deserialize
    let backup: GraphBackup = serde_json::from_str(&json)
        .map_err(|e| JsValue::from_str(&format!("Deserialization failed: {}", e)))?;

    // 6. Restore nodes to GRAPH
    for node in &backup.nodes {
        // Ignore errors for duplicate nodes (they may already exist)
        let _ = GRAPH.create_node(
            &node.vault_id,
            &node.node_type,
            node.encrypted_content.clone(),
            node.content_hmac.clone(),
            node.labels.clone(),
            node.embedding.clone(),
            node.namespace.clone(),
        ).await;
    }

    // 7. Restore edges to GRAPH
    for edge in &backup.edges {
        let _ = GRAPH.create_edge(
            &edge.vault_id,
            &edge.from_node,
            &edge.to_node,
            &edge.edge_type,
            edge.properties.clone(),
        ).await;
    }

    Ok(true)
}
