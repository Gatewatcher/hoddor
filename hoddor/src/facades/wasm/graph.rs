use super::converters;
use crate::adapters::wasm::SimpleGraphAdapter;
use crate::platform::Platform;
use crate::ports::graph::GraphPort;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

static GRAPH: Lazy<SimpleGraphAdapter> = Lazy::new(|| SimpleGraphAdapter::new());

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

#[wasm_bindgen]
pub async fn graph_create_memory_node(
    vault_name: &str,
    encrypted_content: Vec<u8>,
    content_hmac: String,
    embedding: Vec<f32>,
    labels: Vec<String>,
) -> Result<String, JsValue> {
    let vault_id = vault_name;

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

#[wasm_bindgen]
pub async fn graph_vector_search(
    vault_name: &str,
    query_embedding: Vec<f32>,
    limit: usize,
    min_similarity: Option<f32>,
) -> Result<JsValue, JsValue> {
    let vault_id = vault_name;

    let results = GRAPH
        .vector_search(vault_id, query_embedding, limit, min_similarity)
        .await
        .map_err(converters::to_js_error)?;

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

#[wasm_bindgen]
pub async fn graph_get_node(_vault_name: &str, _node_id: &str) -> Result<JsValue, JsValue> {
    Err(JsValue::from_str(
        "graph_get_node not yet implemented - use vector_search instead",
    ))
}

#[wasm_bindgen]
pub async fn graph_list_memory_nodes(
    vault_name: &str,
    limit: Option<usize>,
) -> Result<JsValue, JsValue> {
    let vault_id = vault_name;

    let nodes = GRAPH
        .list_nodes_by_type(vault_id, "memory", limit)
        .await
        .map_err(converters::to_js_error)?;

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

#[wasm_bindgen]
pub async fn graph_backup_vault(
    vault_name: &str,
    recipient: &str,
    _identity: &str,
) -> Result<(), JsValue> {
    use crate::adapters::wasm::graph_persistence::GraphBackup;
    use crate::domain::crypto;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    let mut all_nodes = Vec::new();
    let node_types = vec![
        "memory",
        "entity",
        "event",
        "concept",
        "conversation",
        "document",
        "preference",
    ];

    for node_type in node_types {
        match GRAPH.list_nodes_by_type(vault_name, node_type, None).await {
            Ok(nodes) => all_nodes.extend(nodes),
            Err(_) => continue,
        }
    }

    let mut all_edges = Vec::new();
    let mut seen_edge_ids = std::collections::HashSet::new();

    for node in &all_nodes {
        match GRAPH
            .get_edges(
                vault_name,
                &node.id,
                crate::domain::graph::EdgeDirection::Both,
            )
            .await
        {
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

    let backup = GraphBackup {
        version: 1,
        nodes: all_nodes,
        edges: all_edges,
        created_at: js_sys::Date::now() as u64,
    };

    let json = serde_json::to_string(&backup)
        .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))?;

    let encrypted = crypto::encrypt_for_recipients(&*PLATFORM, json.as_bytes(), &[recipient])
        .await
        .map_err(|e| JsValue::from_str(&format!("Encryption failed: {}", e)))?;

    let data_to_save = BASE64.encode(&encrypted);

    PLATFORM
        .storage()
        .create_directory("graph_backups")
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create directory: {}", e)))?;

    let file_path = format!("graph_backups/{}.age", vault_name);
    PLATFORM
        .storage()
        .write_file(&file_path, &data_to_save)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to write file: {}", e)))?;

    Ok(())
}

#[wasm_bindgen]
pub async fn graph_restore_vault(
    vault_name: &str,
    _recipient: &str,
    identity: &str,
) -> Result<bool, JsValue> {
    use crate::adapters::wasm::graph_persistence::GraphBackup;
    use crate::domain::crypto;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    let file_path = format!("graph_backups/{}.age", vault_name);
    let file_content = match PLATFORM.storage().read_file(&file_path).await {
        Ok(content) => content,
        Err(_) => return Ok(false),
    };

    let encrypted = BASE64
        .decode(&file_content)
        .map_err(|e| JsValue::from_str(&format!("Base64 decode failed: {}", e)))?;

    let decrypted = crypto::decrypt_with_identity(&*PLATFORM, &encrypted, identity)
        .await
        .map_err(|e| JsValue::from_str(&format!("Decryption failed: {}", e)))?;

    let json = String::from_utf8(decrypted)
        .map_err(|e| JsValue::from_str(&format!("UTF-8 conversion failed: {}", e)))?;

    let backup: GraphBackup = serde_json::from_str(&json)
        .map_err(|e| JsValue::from_str(&format!("Deserialization failed: {}", e)))?;

    for node in &backup.nodes {
        let _ = GRAPH
            .create_node(
                &node.vault_id,
                &node.node_type,
                node.encrypted_content.clone(),
                node.content_hmac.clone(),
                node.labels.clone(),
                node.embedding.clone(),
                node.namespace.clone(),
            )
            .await;
    }

    for edge in &backup.edges {
        let _ = GRAPH
            .create_edge(
                &edge.vault_id,
                &edge.from_node,
                &edge.to_node,
                &edge.edge_type,
                edge.properties.clone(),
            )
            .await;
    }

    Ok(true)
}
