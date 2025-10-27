use crate::domain::graph::{
    EdgeDirection, EdgeId, EdgeProperties, GraphBackup, GraphEdge, GraphNode, GraphResult, NodeId,
};
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait GraphPort {
    async fn create_node(
        &self,
        vault_id: &str,
        node_type: &str,
        content: Vec<u8>,
        labels: Vec<String>,
        embedding: Option<Vec<f32>>,
        namespace: Option<String>,
    ) -> GraphResult<NodeId>;

    async fn get_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<Option<GraphNode>>;

    async fn update_node(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        content: Vec<u8>,
        embedding: Option<Vec<f32>>,
    ) -> GraphResult<()>;

    async fn delete_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<()>;

    async fn list_nodes_by_type(
        &self,
        vault_id: &str,
        node_type: &str,
        limit: Option<usize>,
    ) -> GraphResult<Vec<GraphNode>>;

    async fn create_edge(
        &self,
        vault_id: &str,
        from_node: &NodeId,
        to_node: &NodeId,
        edge_type: &str,
        properties: EdgeProperties,
    ) -> GraphResult<EdgeId>;

    async fn get_edges(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        direction: EdgeDirection,
    ) -> GraphResult<Vec<GraphEdge>>;

    async fn delete_edge(&self, vault_id: &str, edge_id: &EdgeId) -> GraphResult<()>;

    async fn get_neighbors(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        edge_types: Option<Vec<String>>,
    ) -> GraphResult<Vec<GraphNode>>;

    async fn vector_search(
        &self,
        vault_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
        min_similarity: Option<f32>,
    ) -> GraphResult<Vec<(GraphNode, f32)>>;

    async fn export_backup(&self, vault_id: &str) -> GraphResult<GraphBackup>;

    async fn import_backup(&self, backup: &GraphBackup) -> GraphResult<()>;
}
