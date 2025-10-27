use crate::domain::graph::{
    create_node_metadata, validate_edge, validate_node, EdgeDirection, EdgeId, EdgeProperties,
    GraphBackup, GraphEdge, GraphError, GraphNode, GraphResult, NodeId,
};
use crate::ports::graph::GraphPort;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Global shared storage for all graph data
static NODES: Lazy<Arc<Mutex<HashMap<NodeId, GraphNode>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
static EDGES: Lazy<Arc<Mutex<HashMap<EdgeId, GraphEdge>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Clone)]
pub struct SimpleGraphAdapter {
    nodes: Arc<Mutex<HashMap<NodeId, GraphNode>>>,
    edges: Arc<Mutex<HashMap<EdgeId, GraphEdge>>>,
}

impl SimpleGraphAdapter {
    pub fn new() -> Self {
        Self {
            nodes: NODES.clone(),
            edges: EDGES.clone(),
        }
    }

    fn get_timestamp() -> u64 {
        js_sys::Date::now() as u64
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let mut dot_product = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..a.len() {
            dot_product += a[i] * b[i];
            norm_a += a[i] * a[i];
            norm_b += b[i] * b[i];
        }

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a.sqrt() * norm_b.sqrt())
    }
}

impl Default for SimpleGraphAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl GraphPort for SimpleGraphAdapter {
    async fn create_node(
        &self,
        vault_id: &str,
        node_type: &str,
        content: Vec<u8>,
        labels: Vec<String>,
        embedding: Option<Vec<f32>>,
        namespace: Option<String>,
    ) -> GraphResult<NodeId> {
        let node_id = NodeId::new();
        let now = Self::get_timestamp();

        let node = GraphNode {
            id: node_id.clone(),
            node_type: node_type.to_string(),
            vault_id: vault_id.to_string(),
            namespace,
            labels,
            embedding,
            content,
            metadata: create_node_metadata(0, None),
            created_at: now,
            updated_at: now,
            accessed_at: now,
            access_count: 0,
        };

        validate_node(&node)?;

        let mut nodes = self.nodes.lock().unwrap();
        nodes.insert(node_id.clone(), node);

        Ok(node_id)
    }

    async fn get_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<Option<GraphNode>> {
        let nodes = self.nodes.lock().unwrap();

        if let Some(node) = nodes.get(node_id) {
            if node.vault_id == vault_id {
                Ok(Some(node.clone()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn update_node(
        &self,
        _vault_id: &str,
        node_id: &NodeId,
        content: Vec<u8>,
        embedding: Option<Vec<f32>>,
    ) -> GraphResult<()> {
        let now = Self::get_timestamp();
        let mut nodes = self.nodes.lock().unwrap();

        if let Some(node) = nodes.get_mut(node_id) {
            node.content = content;
            node.embedding = embedding;
            node.updated_at = now;
            Ok(())
        } else {
            Err(GraphError::Other("Node not found".to_string()))
        }
    }

    async fn delete_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<()> {
        let mut edges = self.edges.lock().unwrap();
        edges.retain(|_, edge| {
            !(edge.vault_id == vault_id && (edge.from_node == *node_id || edge.to_node == *node_id))
        });

        let mut nodes = self.nodes.lock().unwrap();
        if nodes.remove(node_id).is_some() {
            Ok(())
        } else {
            Err(GraphError::Other("Node not found".to_string()))
        }
    }

    async fn list_nodes_by_type(
        &self,
        vault_id: &str,
        node_type: &str,
        limit: Option<usize>,
    ) -> GraphResult<Vec<GraphNode>> {
        let nodes = self.nodes.lock().unwrap();

        let mut result: Vec<GraphNode> = nodes
            .values()
            .filter(|node| node.vault_id == vault_id && node.node_type == node_type)
            .cloned()
            .collect();

        if let Some(lim) = limit {
            result.truncate(lim);
        }

        Ok(result)
    }

    async fn create_edge(
        &self,
        vault_id: &str,
        from_node: &NodeId,
        to_node: &NodeId,
        edge_type: &str,
        properties: EdgeProperties,
    ) -> GraphResult<EdgeId> {
        let edge_id = EdgeId::new();
        let now = Self::get_timestamp();

        let edge = GraphEdge {
            id: edge_id.clone(),
            from_node: from_node.clone(),
            to_node: to_node.clone(),
            edge_type: edge_type.to_string(),
            vault_id: vault_id.to_string(),
            properties,
            created_at: now,
        };

        validate_edge(&edge)?;

        let mut edges = self.edges.lock().unwrap();
        edges.insert(edge_id.clone(), edge);

        Ok(edge_id)
    }

    async fn get_edges(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        direction: EdgeDirection,
    ) -> GraphResult<Vec<GraphEdge>> {
        let edges = self.edges.lock().unwrap();

        let result: Vec<GraphEdge> = edges
            .values()
            .filter(|edge| {
                edge.vault_id == vault_id
                    && match direction {
                        EdgeDirection::Outgoing => edge.from_node == *node_id,
                        EdgeDirection::Incoming => edge.to_node == *node_id,
                        EdgeDirection::Both => {
                            edge.from_node == *node_id || edge.to_node == *node_id
                        }
                    }
            })
            .cloned()
            .collect();

        Ok(result)
    }

    async fn delete_edge(&self, vault_id: &str, edge_id: &EdgeId) -> GraphResult<()> {
        let mut edges = self.edges.lock().unwrap();

        if let Some(edge) = edges.get(edge_id) {
            if edge.vault_id == vault_id {
                edges.remove(edge_id);
                Ok(())
            } else {
                Err(GraphError::Other("Edge not found in vault".to_string()))
            }
        } else {
            Err(GraphError::Other("Edge not found".to_string()))
        }
    }

    async fn get_neighbors(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        edge_types: Option<Vec<String>>,
    ) -> GraphResult<Vec<GraphNode>> {
        let edges = self.edges.lock().unwrap();
        let nodes = self.nodes.lock().unwrap();

        let neighbor_ids: Vec<NodeId> = edges
            .values()
            .filter(|edge| {
                edge.vault_id == vault_id
                    && (edge.from_node == *node_id || edge.to_node == *node_id)
                    && edge_types
                        .as_ref()
                        .map_or(true, |types| types.contains(&edge.edge_type))
            })
            .map(|edge| {
                if edge.from_node == *node_id {
                    edge.to_node.clone()
                } else {
                    edge.from_node.clone()
                }
            })
            .collect();

        let result: Vec<GraphNode> = neighbor_ids
            .iter()
            .filter_map(|id| nodes.get(id).cloned())
            .collect();

        Ok(result)
    }

    async fn vector_search(
        &self,
        vault_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
        min_similarity: Option<f32>,
    ) -> GraphResult<Vec<(GraphNode, f32)>> {
        let nodes = self.nodes.lock().unwrap();

        let mut results: Vec<(GraphNode, f32)> = nodes
            .values()
            .filter(|node| node.vault_id == vault_id && node.embedding.is_some())
            .map(|node| {
                let similarity =
                    Self::cosine_similarity(&query_embedding, node.embedding.as_ref().unwrap());
                (node.clone(), similarity)
            })
            .filter(|(_, similarity)| {
                if let Some(min_sim) = min_similarity {
                    *similarity >= min_sim
                } else {
                    true
                }
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        results.truncate(limit);

        Ok(results)
    }

    async fn export_backup(&self, vault_id: &str) -> GraphResult<GraphBackup> {
        let nodes = self.nodes.lock().unwrap();
        let edges = self.edges.lock().unwrap();

        let vault_nodes: Vec<GraphNode> = nodes
            .values()
            .filter(|node| node.vault_id == vault_id)
            .cloned()
            .collect();

        let vault_edges: Vec<GraphEdge> = edges
            .values()
            .filter(|edge| edge.vault_id == vault_id)
            .cloned()
            .collect();

        Ok(GraphBackup {
            version: 1,
            nodes: vault_nodes,
            edges: vault_edges,
            created_at: js_sys::Date::now() as u64,
        })
    }

    async fn import_backup(&self, backup: &GraphBackup) -> GraphResult<()> {
        let mut nodes = self.nodes.lock().unwrap();
        let mut edges = self.edges.lock().unwrap();

        for node in &backup.nodes {
            nodes.insert(node.id.clone(), node.clone());
        }

        for edge in &backup.edges {
            edges.insert(edge.id.clone(), edge.clone());
        }

        Ok(())
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::SimpleGraphAdapter;
    use crate::domain::graph::{EdgeDirection, EdgeProperties, EdgeType, NodeId};
    use crate::ports::graph::GraphPort;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_create_graph_adapter() {
        let adapter = SimpleGraphAdapter::new();
        assert!(adapter.nodes.lock().is_ok());
        assert!(adapter.edges.lock().is_ok());
    }

    #[wasm_bindgen_test]
    async fn test_create_and_get_node() {
        let adapter = SimpleGraphAdapter::new();

        let content = vec![1, 2, 3, 4, 5];
        let node_id = adapter
            .create_node(
                "test_vault",
                "memory",
                content.clone(),
                vec!["test".to_string(), "integration".to_string()],
                Some(vec![0.1, 0.2, 0.3]),
                Some("test_namespace".to_string()),
            )
            .await
            .expect("Failed to create node");

        let retrieved = adapter
            .get_node("test_vault", &node_id)
            .await
            .expect("Failed to get node");

        assert!(retrieved.is_some());
        let node = retrieved.unwrap();

        assert_eq!(node.id, node_id);
        assert_eq!(node.node_type, "memory");
        assert_eq!(node.vault_id, "test_vault");
        assert_eq!(node.namespace, Some("test_namespace".to_string()));
        assert_eq!(node.labels, vec!["test", "integration"]);
        assert_eq!(node.content, content);
        assert!(node.embedding.is_some());
    }

    #[wasm_bindgen_test]
    async fn test_create_update_get_node() {
        let adapter = SimpleGraphAdapter::new();

        let node_id = adapter
            .create_node(
                "test_vault",
                "memory",
                vec![1, 2, 3],
                vec!["original".to_string()],
                None,
                None,
            )
            .await
            .unwrap();

        let new_content = vec![4, 5, 6, 7];
        adapter
            .update_node(
                "test_vault",
                &node_id,
                new_content.clone(),
                Some(vec![0.5, 0.6]),
            )
            .await
            .expect("Failed to update node");

        let node = adapter
            .get_node("test_vault", &node_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(node.content, new_content);
        assert_eq!(node.embedding, Some(vec![0.5, 0.6]));
    }

    #[wasm_bindgen_test]
    async fn test_create_and_delete_node() {
        let adapter = SimpleGraphAdapter::new();

        let node_id = adapter
            .create_node("test_vault", "memory", vec![1, 2, 3], vec![], None, None)
            .await
            .unwrap();

        assert!(adapter
            .get_node("test_vault", &node_id)
            .await
            .unwrap()
            .is_some());

        adapter
            .delete_node("test_vault", &node_id)
            .await
            .expect("Failed to delete node");

        assert!(adapter
            .get_node("test_vault", &node_id)
            .await
            .unwrap()
            .is_none());
    }

    #[wasm_bindgen_test]
    async fn test_create_and_get_edges() {
        let adapter = SimpleGraphAdapter::new();

        let from_id = adapter
            .create_node("test_vault", "entity", vec![1], vec![], None, None)
            .await
            .unwrap();

        let to_id = adapter
            .create_node("test_vault", "entity", vec![2], vec![], None, None)
            .await
            .unwrap();

        let edge_id = adapter
            .create_edge(
                "test_vault",
                &from_id,
                &to_id,
                EdgeType::RelatesTo.as_str(),
                EdgeProperties {
                    weight: 0.8,
                    bidirectional: false,
                    encrypted_context: None,
                    metadata: Default::default(),
                },
            )
            .await
            .expect("Failed to create edge");

        let outgoing = adapter
            .get_edges("test_vault", &from_id, EdgeDirection::Outgoing)
            .await
            .unwrap();

        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].id, edge_id);
        assert_eq!(outgoing[0].from_node, from_id);
        assert_eq!(outgoing[0].to_node, to_id);
        assert_eq!(outgoing[0].edge_type, "relates_to");
        assert_eq!(outgoing[0].properties.weight, 0.8);

        let incoming = adapter
            .get_edges("test_vault", &to_id, EdgeDirection::Incoming)
            .await
            .unwrap();

        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].id, edge_id);
    }

    #[wasm_bindgen_test]
    async fn test_get_neighbors() {
        let adapter = SimpleGraphAdapter::new();

        let node_a = adapter
            .create_node("test_vault", "entity", vec![1], vec![], None, None)
            .await
            .unwrap();

        let node_b = adapter
            .create_node("test_vault", "entity", vec![2], vec![], None, None)
            .await
            .unwrap();

        let node_c = adapter
            .create_node("test_vault", "entity", vec![3], vec![], None, None)
            .await
            .unwrap();

        adapter
            .create_edge(
                "test_vault",
                &node_a,
                &node_b,
                "relates_to",
                EdgeProperties::default(),
            )
            .await
            .unwrap();

        adapter
            .create_edge(
                "test_vault",
                &node_b,
                &node_c,
                "relates_to",
                EdgeProperties::default(),
            )
            .await
            .unwrap();

        let neighbors = adapter
            .get_neighbors("test_vault", &node_b, None)
            .await
            .unwrap();

        assert_eq!(neighbors.len(), 2);
        let neighbor_ids: Vec<NodeId> = neighbors.iter().map(|n| n.id.clone()).collect();
        assert!(neighbor_ids.contains(&node_a));
        assert!(neighbor_ids.contains(&node_c));
    }

    #[wasm_bindgen_test]
    async fn test_delete_node_with_edges() {
        let adapter = SimpleGraphAdapter::new();

        let node_a = adapter
            .create_node("test_vault", "entity", vec![1], vec![], None, None)
            .await
            .unwrap();

        let node_b = adapter
            .create_node("test_vault", "entity", vec![2], vec![], None, None)
            .await
            .unwrap();

        adapter
            .create_edge(
                "test_vault",
                &node_a,
                &node_b,
                "relates_to",
                EdgeProperties::default(),
            )
            .await
            .unwrap();

        let edges = adapter
            .get_edges("test_vault", &node_a, EdgeDirection::Outgoing)
            .await
            .unwrap();
        assert_eq!(edges.len(), 1);

        adapter.delete_node("test_vault", &node_a).await.unwrap();

        assert!(adapter
            .get_node("test_vault", &node_a)
            .await
            .unwrap()
            .is_none());

        let edges = adapter
            .get_edges("test_vault", &node_b, EdgeDirection::Incoming)
            .await
            .unwrap();
        assert_eq!(edges.len(), 0);
    }
}
