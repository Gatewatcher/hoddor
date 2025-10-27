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
    pub content: Vec<u8>,
    pub labels: Vec<String>,
    pub similarity: Option<f32>,
}

#[wasm_bindgen]
pub async fn graph_create_memory_node(
    vault_name: &str,
    content: Vec<u8>,
    embedding: Vec<f32>,
    labels: Vec<String>,
) -> Result<String, JsValue> {
    let vault_id = vault_name;

    let node_id = GRAPH
        .create_node(
            vault_id,
            "memory",
            content,
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
            content: node.content,
            labels: node.labels,
            similarity: Some(similarity),
        })
        .collect();

    serde_wasm_bindgen::to_value(&js_results).map_err(converters::to_js_error)
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
            content: node.content,
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
    identity: &str,
) -> Result<(), JsValue> {
    use crate::adapters::wasm::OpfsStorage;
    use crate::domain::graph::{EncryptionConfig, GraphPersistenceService};

    let encryption = EncryptionConfig {
        platform: PLATFORM.clone(),
        recipient: recipient.to_string(),
        identity: identity.to_string(),
    };

    let service = GraphPersistenceService::new_with_encryption(
        &*GRAPH,
        OpfsStorage::new(),
        "graph_backups".to_string(),
        encryption,
    );

    service
        .backup(vault_name)
        .await
        .map_err(converters::to_js_error)
}

#[wasm_bindgen]
pub async fn graph_restore_vault(
    vault_name: &str,
    recipient: &str,
    identity: &str,
) -> Result<bool, JsValue> {
    use crate::adapters::wasm::OpfsStorage;
    use crate::domain::graph::{EncryptionConfig, GraphPersistenceService};

    let encryption = EncryptionConfig {
        platform: PLATFORM.clone(),
        recipient: recipient.to_string(),
        identity: identity.to_string(),
    };

    let service = GraphPersistenceService::new_with_encryption(
        &*GRAPH,
        OpfsStorage::new(),
        "graph_backups".to_string(),
        encryption,
    );

    if !service.backup_exists(vault_name).await {
        return Ok(false);
    }
    
    service
        .restore(vault_name)
        .await
        .map_err(converters::to_js_error)?;

    Ok(true)
}
