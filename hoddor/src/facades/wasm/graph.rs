use super::converters;
use crate::adapters::wasm::SimpleGraphAdapter;
use crate::domain::graph::NodeId;
use crate::ports::graph::GraphPort;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// Global singleton graph adapter - shared across all WASM function calls
static GRAPH: Lazy<SimpleGraphAdapter> = Lazy::new(|| SimpleGraphAdapter::new());

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
