use crate::domain::graph::{
    EdgeDirection, EdgeId, EdgeProperties, GraphEdge, GraphNode, GraphResult, NodeId,
};
use async_trait::async_trait;

/// Port for graph database operations
#[async_trait(?Send)]
pub trait GraphPort {
    // ═══════════════════════════════════════
    // Node Operations
    // ═══════════════════════════════════════

    /// Create a new node in the graph
    async fn create_node(
        &self,
        vault_id: &str,
        node_type: &str,
        encrypted_content: Vec<u8>,
        content_hmac: String,
        labels: Vec<String>,
        embedding: Option<Vec<f32>>,
        namespace: Option<String>,
    ) -> GraphResult<NodeId>;

    /// Get a node by its ID
    async fn get_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<Option<GraphNode>>;

    /// Update a node's content
    async fn update_node(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        encrypted_content: Vec<u8>,
        content_hmac: String,
        embedding: Option<Vec<f32>>,
    ) -> GraphResult<()>;

    /// Delete a node and all its edges
    async fn delete_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<()>;

    /// List all nodes of a specific type
    async fn list_nodes_by_type(
        &self,
        vault_id: &str,
        node_type: &str,
        limit: Option<usize>,
    ) -> GraphResult<Vec<GraphNode>>;

    // ═══════════════════════════════════════
    // Edge Operations
    // ═══════════════════════════════════════

    /// Create a new edge between two nodes
    async fn create_edge(
        &self,
        vault_id: &str,
        from_node: &NodeId,
        to_node: &NodeId,
        edge_type: &str,
        properties: EdgeProperties,
    ) -> GraphResult<EdgeId>;

    /// Get all edges connected to a node
    async fn get_edges(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        direction: EdgeDirection,
    ) -> GraphResult<Vec<GraphEdge>>;

    /// Delete an edge
    async fn delete_edge(&self, vault_id: &str, edge_id: &EdgeId) -> GraphResult<()>;

    // ═══════════════════════════════════════
    // Query Operations (basic for MVP)
    // ═══════════════════════════════════════

    /// Get neighboring nodes (1 level deep)
    async fn get_neighbors(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        edge_types: Option<Vec<String>>,
    ) -> GraphResult<Vec<GraphNode>>;
}
