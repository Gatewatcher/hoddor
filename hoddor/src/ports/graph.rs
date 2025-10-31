use crate::domain::graph::{GraphBackup, GraphNode, GraphResult, Id, SearchResult};
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait GraphPort {
    async fn create_node(
        &self,
        vault_id: &str,
        node_type: &str,
        content: String,
        labels: Vec<String>,
        embedding: Option<Vec<f32>>,
        node_id: Option<&Id>,
    ) -> GraphResult<Id>;

    async fn list_nodes_by_type(
        &self,
        vault_id: &str,
        node_type: &str,
        limit: Option<usize>,
    ) -> GraphResult<Vec<GraphNode>>;

    async fn create_edge(
        &self,
        vault_id: &str,
        from_node: &Id,
        to_node: &Id,
        edge_type: &str,
        weight: Option<f32>,
        edge_id: Option<&Id>,
    ) -> GraphResult<Id>;

    async fn vector_search_with_neighbors(
        &self,
        vault_id: &str,
        query_embedding: Vec<f32>,
        max_results: usize,
        search_quality: usize,
        include_neighbors: bool,
    ) -> GraphResult<Vec<SearchResult>>;

    async fn export_backup(&self, vault_id: &str) -> GraphResult<GraphBackup>;
    async fn import_backup(&self, backup: &GraphBackup) -> GraphResult<()>;
}
