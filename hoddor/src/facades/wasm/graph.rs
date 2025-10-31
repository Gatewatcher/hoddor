use super::converters;
use crate::domain::graph::Id;
use crate::platform::Platform;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct GraphNodeResult {
    pub id: String,
    pub node_type: String,
    pub content: String,
    pub labels: Vec<String>,
    pub similarity: Option<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct GraphNodeWithNeighborsResult {
    pub id: String,
    pub node_type: String,
    pub content: String,
    pub labels: Vec<String>,
    pub similarity: f32,
    pub neighbors: Vec<GraphNodeResult>,
}

#[wasm_bindgen]
pub async fn graph_create_memory_node(
    vault_name: &str,
    content: String,
    embedding: Vec<f32>,
    labels: Vec<String>,
) -> Result<String, JsValue> {
    let platform = Platform::new();
    let vault_id = vault_name;

    let node_id = platform
        .graph()
        .create_node(vault_id, "memory", content, labels, Some(embedding), None)
        .await
        .map_err(converters::to_js_error)?;

    Ok(node_id.as_str().to_string())
}

#[wasm_bindgen]
pub async fn graph_vector_search(
    vault_name: &str,
    query_embedding: Vec<f32>,
    max_results: usize,
    search_quality: usize,
) -> Result<JsValue, JsValue> {
    let platform = Platform::new();
    let vault_id = vault_name;

    let results = platform
        .graph()
        .vector_search_with_neighbors(
            vault_id,
            query_embedding,
            max_results,
            search_quality,
            false,
        )
        .await
        .map_err(converters::to_js_error)?;

    let js_results: Vec<GraphNodeResult> = results
        .into_iter()
        .map(|search_result| GraphNodeResult {
            id: search_result.node.id.as_str().to_string(),
            node_type: search_result.node.node_type,
            content: search_result.node.content,
            labels: search_result.node.labels,
            similarity: Some(search_result.distance),
        })
        .collect();

    serde_wasm_bindgen::to_value(&js_results).map_err(converters::to_js_error)
}

#[wasm_bindgen]
pub async fn graph_list_memory_nodes(
    vault_name: &str,
    limit: Option<usize>,
) -> Result<JsValue, JsValue> {
    let platform = Platform::new();
    let vault_id = vault_name;

    let nodes = platform
        .graph()
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
pub async fn graph_create_edge(
    vault_name: &str,
    from_node_id: &str,
    to_node_id: &str,
    edge_type: &str,
    weight: Option<f32>,
) -> Result<String, JsValue> {
    let platform = Platform::new();
    let vault_id = vault_name;

    let from_node = Id::from_string(from_node_id)
        .map_err(|e| JsValue::from_str(&format!("Invalid from_node_id: {}", e)))?;

    let to_node = Id::from_string(to_node_id)
        .map_err(|e| JsValue::from_str(&format!("Invalid to_node_id: {}", e)))?;

    let edge_id = platform
        .graph()
        .create_edge(vault_id, &from_node, &to_node, edge_type, weight, None)
        .await
        .map_err(converters::to_js_error)?;

    Ok(edge_id.as_str().to_string())
}

#[wasm_bindgen]
pub async fn graph_vector_search_with_neighbors(
    vault_name: &str,
    query_embedding: Vec<f32>,
    max_results: usize,
    search_quality: usize,
) -> Result<JsValue, JsValue> {
    let platform = Platform::new();
    let vault_id = vault_name;

    let results = platform
        .graph()
        .vector_search_with_neighbors(vault_id, query_embedding, max_results, search_quality, true)
        .await
        .map_err(converters::to_js_error)?;

    let js_results: Vec<GraphNodeWithNeighborsResult> = results
        .into_iter()
        .map(|search_result| GraphNodeWithNeighborsResult {
            id: search_result.node.id.as_str().to_string(),
            node_type: search_result.node.node_type,
            content: search_result.node.content,
            labels: search_result.node.labels,
            similarity: search_result.distance,
            neighbors: search_result
                .neighbors
                .into_iter()
                .map(|neighbor| GraphNodeResult {
                    id: neighbor.node.id.as_str().to_string(),
                    node_type: neighbor.node.node_type,
                    content: neighbor.node.content,
                    labels: neighbor.node.labels,
                    similarity: None,
                })
                .collect(),
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
    use crate::domain::graph::{EncryptionConfig, GraphPersistenceService};

    let platform = Platform::new();

    let encryption = EncryptionConfig {
        platform: platform.clone(),
        recipient: recipient.to_string(),
        identity: identity.to_string(),
    };

    let service = GraphPersistenceService::new(
        platform.graph_owned(),
        platform.storage_owned(),
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
    use crate::domain::graph::{EncryptionConfig, GraphPersistenceService};

    let platform = Platform::new();

    let encryption = EncryptionConfig {
        platform: platform.clone(),
        recipient: recipient.to_string(),
        identity: identity.to_string(),
    };

    let service = GraphPersistenceService::new(
        platform.graph_owned(),
        platform.storage_owned(),
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
