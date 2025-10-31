
use crate::domain::graph::{
    GraphBackup, GraphEdge, GraphError, GraphNode, GraphResult, Id, NeighborNode, SearchResult,
};
use crate::ports::graph::GraphPort;
use async_trait::async_trait;
use cozo::{DataValue, DbInstance, ScriptMutability, Vector};
use ndarray::Array1;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// HNSW Index Configuration
// ========================
// DEFAULT_EMBEDDING_DIM: Vector dimension (384 for sentence-transformers models)
// HNSW_M: Number of bi-directional links per node (16-24 recommended, higher = more memory)
// HNSW_EF_CONSTRUCTION: Size of candidate list during index building (higher = better quality, slower build)
const DEFAULT_EMBEDDING_DIM: usize = 384;
const HNSW_M: i64 = 16;
const HNSW_EF_CONSTRUCTION: i64 = 200;

static GLOBAL_COZO_DB: Lazy<Arc<Mutex<DbInstance>>> = Lazy::new(|| {
    let db = DbInstance::new("mem", "", Default::default())
        .expect("Failed to create global CozoDB instance");
    Arc::new(Mutex::new(db))
});

// Helper functions for data conversion
fn labels_to_string(labels: &[String]) -> String {
    labels.join(",")
}

fn string_to_labels(s: &str) -> Vec<String> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split(',').map(|s| s.to_string()).collect()
    }
}

fn vec_f32_to_datavalue(vec: Option<Vec<f32>>) -> DataValue {
    match vec {
        Some(v) => {
            let arr = Array1::from_vec(v);
            DataValue::Vec(Vector::F32(arr))
        }
        None => DataValue::Null,
    }
}

impl TryFrom<Vec<DataValue>> for GraphNode {
    type Error = GraphError;

    fn try_from(row: Vec<DataValue>) -> Result<Self, Self::Error> {
        if row.len() < 7 {
            return Err(GraphError::DatabaseError("Invalid row format".to_string()));
        }

        Ok(GraphNode {
            id: Id::from_string(
                row[0]
                    .get_str()
                    .ok_or_else(|| GraphError::DatabaseError("Missing id".to_string()))?,
            )
            .map_err(|e| GraphError::DatabaseError(format!("Invalid id: {}", e)))?,
            node_type: row[1].get_str().unwrap_or("unknown").to_string(),
            vault_id: row[2].get_str().unwrap_or("").to_string(),
            content: row[3].get_str().unwrap_or("").to_string(),
            labels: string_to_labels(row[4].get_str().unwrap_or("")),
            embedding: match &row[5] {
                DataValue::Vec(Vector::F32(arr)) => Some(arr.to_vec()),
                _ => None,
            },
            created_at: row[6].get_int().unwrap_or(0) as u64,
        })
    }
}

impl TryFrom<Vec<DataValue>> for GraphEdge {
    type Error = GraphError;

    fn try_from(row: Vec<DataValue>) -> Result<Self, Self::Error> {
        if row.len() < 7 {
            return Err(GraphError::DatabaseError(format!(
                "Invalid edge row: expected 7 columns, got {}",
                row.len()
            )));
        }

        Ok(GraphEdge {
            id: Id::from_string(
                row[0]
                    .get_str()
                    .ok_or_else(|| GraphError::DatabaseError("Missing edge id".to_string()))?,
            )
            .map_err(|e| GraphError::DatabaseError(format!("Invalid edge id: {}", e)))?,
            from_node: Id::from_string(
                row[1]
                    .get_str()
                    .ok_or_else(|| GraphError::DatabaseError("Missing from_node".to_string()))?,
            )
            .map_err(|e| GraphError::DatabaseError(format!("Invalid from_node: {}", e)))?,
            to_node: Id::from_string(
                row[2]
                    .get_str()
                    .ok_or_else(|| GraphError::DatabaseError("Missing to_node".to_string()))?,
            )
            .map_err(|e| GraphError::DatabaseError(format!("Invalid to_node: {}", e)))?,
            edge_type: row[3].get_str().unwrap_or("unknown").to_string(),
            vault_id: row[4].get_str().unwrap_or("").to_string(),
            weight: row[5].get_float().unwrap_or(1.0) as f32,
            created_at: row[6].get_int().unwrap_or(0) as u64,
        })
    }
}

#[derive(Clone)]
pub struct CozoGraphAdapter {
    db: Arc<Mutex<DbInstance>>,
}

impl CozoGraphAdapter {
    pub fn new() -> GraphResult<Self> {
        static SCHEMA_INITIALIZED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

        let adapter = Self {
            db: GLOBAL_COZO_DB.clone(),
        };

        let mut initialized = SCHEMA_INITIALIZED
            .lock()
            .map_err(|e| GraphError::DatabaseError(format!("Lock error: {}", e)))?;

        if !*initialized {
            adapter.init_schema()?;
            *initialized = true;
        }

        Ok(adapter)
    }

    fn init_schema(&self) -> GraphResult<()> {
        let db = self
            .db
            .lock()
            .map_err(|e| GraphError::DatabaseError(format!("Lock error: {}", e)))?;

        let schema_nodes = format!(
            r#"
            :create nodes {{
                id: String =>
                node_type: String,
                vault_id: String,
                content: String,
                labels: String,
                embedding: <F32; {}>?,
                created_at: Int,
            }}
            "#,
            DEFAULT_EMBEDDING_DIM
        );

        db.run_script(&schema_nodes, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| {
                GraphError::DatabaseError(format!("Failed to create nodes relation: {}", e))
            })?;

        let schema_edges = r#"
            :create edges {
                id: String =>
                from_node: String,
                to_node: String,
                edge_type: String,
                vault_id: String,
                weight: Float,
                created_at: Int,
            }
        "#;

        db.run_script(schema_edges, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| {
                GraphError::DatabaseError(format!("Failed to create edges relation: {}", e))
            })?;

        let hnsw_index = format!(
            r#"
            ::hnsw create nodes:embedding_idx {{
                dim: {},
                m: {},
                dtype: F32,
                fields: [embedding],
                distance: Cosine,
                ef_construction: {},
            }}
            "#,
            DEFAULT_EMBEDDING_DIM, HNSW_M, HNSW_EF_CONSTRUCTION
        );

        db.run_script(&hnsw_index, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| {
                GraphError::DatabaseError(format!("Failed to create HNSW index: {}", e))
            })?;

        Ok(())
    }

    fn get_timestamp() -> u64 {
        js_sys::Date::now() as u64
    }

    fn parse_simple_search_results(rows: Vec<Vec<DataValue>>) -> GraphResult<Vec<SearchResult>> {
        let mut results = Vec::new();

        for row in rows {
            if row.len() < 5 {
                continue;
            }

            let node_id = Id::from_string(
                row[0]
                    .get_str()
                    .ok_or_else(|| GraphError::DatabaseError("Missing id".to_string()))?,
            )
            .map_err(|e| GraphError::DatabaseError(format!("Invalid id: {}", e)))?;

            let distance = row[4].get_float().unwrap_or(0.0) as f32;

            results.push(SearchResult {
                node: GraphNode {
                    id: node_id,
                    node_type: row[1].get_str().unwrap_or("").to_string(),
                    vault_id: String::new(),
                    content: row[2].get_str().unwrap_or("").to_string(),
                    labels: string_to_labels(row[3].get_str().unwrap_or("")),
                    embedding: None,
                    created_at: 0,
                },
                distance,
                neighbors: Vec::new(),
            });
        }

        Ok(results)
    }

    fn parse_search_results_with_neighbors(
        rows: Vec<Vec<DataValue>>,
    ) -> GraphResult<Vec<SearchResult>> {
        use std::collections::HashMap;

        let mut node_map: HashMap<String, SearchResult> = HashMap::new();

        for row in rows {
            if row.len() < 10 {
                continue;
            }

            let node_id_str = row[0]
                .get_str()
                .ok_or_else(|| GraphError::DatabaseError("Missing id".to_string()))?;

            let distance = row[4].get_float().unwrap_or(0.0) as f32;

            let entry = node_map
                .entry(node_id_str.to_string())
                .or_insert_with(|| SearchResult {
                    node: GraphNode {
                        id: Id::from_string(node_id_str).unwrap(),
                        node_type: row[1].get_str().unwrap_or("").to_string(),
                        vault_id: String::new(),
                        content: row[2].get_str().unwrap_or("").to_string(),
                        labels: string_to_labels(row[3].get_str().unwrap_or("")),
                        embedding: None,
                        created_at: 0,
                    },
                    distance,
                    neighbors: Vec::new(),
                });

            if let Some(neighbor_id_str) = row[5].get_str() {
                let neighbor = NeighborNode {
                    node: GraphNode {
                        id: Id::from_string(neighbor_id_str).unwrap(),
                        node_type: row[6].get_str().unwrap_or("").to_string(),
                        vault_id: String::new(),
                        content: row[7].get_str().unwrap_or("").to_string(),
                        labels: Vec::new(),
                        embedding: None,
                        created_at: 0,
                    },
                    edge_type: row[8].get_str().unwrap_or("").to_string(),
                    weight: row[9].get_float().unwrap_or(1.0) as f32,
                };

                entry.neighbors.push(neighbor);
            }
        }

        let mut results: Vec<SearchResult> = node_map.into_values().collect();
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        Ok(results)
    }
}

impl Default for CozoGraphAdapter {
    fn default() -> Self {
        Self::new().expect("Failed to create CozoGraphAdapter")
    }
}

#[async_trait(?Send)]
impl GraphPort for CozoGraphAdapter {
    async fn create_node(
        &self,
        vault_id: &str,
        node_type: &str,
        content: String,
        labels: Vec<String>,
        embedding: Option<Vec<f32>>,
        node_id: Option<&Id>,
    ) -> GraphResult<Id> {
        let node_id = node_id.unwrap_or(&Id::new()).clone();
        let now = Self::get_timestamp() as i64;

        let db = self
            .db
            .lock()
            .map_err(|e| GraphError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(node_id.as_str().into()));
        params.insert("node_type".to_string(), DataValue::Str(node_type.into()));
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert("content".to_string(), DataValue::Str(content.into()));
        params.insert(
            "labels".to_string(),
            DataValue::Str(labels_to_string(&labels).into()),
        );
        params.insert("embedding".to_string(), vec_f32_to_datavalue(embedding));
        params.insert("created_at".to_string(), DataValue::from(now));

        let query = r#"
            ?[id, node_type, vault_id, content, labels, embedding, created_at] <- [[$id, $node_type, $vault_id, $content, $labels, $embedding, $created_at]]
            :put nodes { id => node_type, vault_id, content, labels, embedding, created_at }
        "#;

        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to create node: {}", e)))?;

        Ok(node_id)
    }

    async fn list_nodes_by_type(
        &self,
        vault_id: &str,
        node_type: &str,
        limit: Option<usize>,
    ) -> GraphResult<Vec<GraphNode>> {
        let db = self
            .db
            .lock()
            .map_err(|e| GraphError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut params = BTreeMap::new();
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert("node_type".to_string(), DataValue::Str(node_type.into()));
        params.insert(
            "limit".to_string(),
            DataValue::from(limit.unwrap_or(100) as i64),
        );

        let query = r#"
            ?[id, node_type, vault_id, content, labels, embedding, created_at] :=
                *nodes{
                    id, node_type, vault_id, content, 
                    labels, embedding, created_at
                },
                node_type == $node_type,
                vault_id == $vault_id
            :limit $limit
        "#;

        let result = db
            .run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to list nodes: {}", e)))?;

        result.rows.into_iter().map(GraphNode::try_from).collect()
    }

    async fn create_edge(
        &self,
        vault_id: &str,
        from_node: &Id,
        to_node: &Id,
        edge_type: &str,
        weight: Option<f32>,
        edge_id: Option<&Id>,
    ) -> GraphResult<Id> {
        let edge_id = edge_id.unwrap_or(&Id::new()).clone();
        let now = Self::get_timestamp() as i64;

        let db = self
            .db
            .lock()
            .map_err(|e| GraphError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(edge_id.as_str().into()));
        params.insert(
            "from_node".to_string(),
            DataValue::Str(from_node.as_str().into()),
        );
        params.insert(
            "to_node".to_string(),
            DataValue::Str(to_node.as_str().into()),
        );
        params.insert("edge_type".to_string(), DataValue::Str(edge_type.into()));
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert(
            "weight".to_string(),
            DataValue::from(weight.unwrap_or(1.0) as f64),
        );
        params.insert("created_at".to_string(), DataValue::from(now));

        let query = r#"
            ?[id, from_node, to_node, edge_type, vault_id, weight, created_at] <- [[$id, $from_node, $to_node, $edge_type, $vault_id, $weight, $created_at]]
            :put edges { id => from_node, to_node, edge_type, vault_id, weight, created_at }
        "#;

        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to create edge: {}", e)))?;

        Ok(edge_id)
    }

    async fn vector_search_with_neighbors(
        &self,
        vault_id: &str,
        query_embedding: Vec<f32>,
        max_results: usize,
        search_quality: usize,
        include_neighbors: bool,
    ) -> GraphResult<Vec<SearchResult>> {
        if query_embedding.len() != DEFAULT_EMBEDDING_DIM {
            return Err(GraphError::InvalidEmbedding(format!(
                "Expected {} dimensions, got {}",
                DEFAULT_EMBEDDING_DIM,
                query_embedding.len()
            )));
        }

        let db = self
            .db
            .lock()
            .map_err(|e| GraphError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut params = BTreeMap::new();
        params.insert(
            "query_vec".to_string(),
            vec_f32_to_datavalue(Some(query_embedding)),
        );
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert(
            "max_results".to_string(),
            DataValue::from(max_results as i64),
        );
        params.insert(
            "search_quality".to_string(),
            DataValue::from(search_quality as i64),
        );

        let query = if include_neighbors {
            r#"
            similar_nodes[id, dist] :=
                ~nodes:embedding_idx{
                    id, embedding |
                    query: $query_vec,
                    k: $max_results,
                    ef: $search_quality,
                    bind_distance: dist
                },
                *nodes{id, vault_id},
                vault_id == $vault_id
            
            ?[
                id, node_type, content, labels, dist,
                neighbor_id, neighbor_type, neighbor_content, edge_type, weight
            ] :=
                similar_nodes[id, dist],
                *nodes{
                    id, 
                    node_type, 
                    content, 
                    labels
                },
                *edges{from_node, to_node, edge_type, weight, vault_id: edge_vault},
                edge_vault == $vault_id,
                (
                    (from_node == id, neighbor_id = to_node) or 
                    (to_node == id, neighbor_id = from_node)
                ),
                neighbor_id != id,
                *nodes{
                    id: neighbor_id, 
                    node_type: neighbor_type,
                    content: neighbor_content
                }
            
            :order dist
        "#
        } else {
            r#"
            ?[id, node_type, content, labels, dist] :=
                ~nodes:embedding_idx{
                    id, embedding |
                    query: $query_vec,
                    k: $max_results,
                    ef: $search_quality,
                    bind_distance: dist
                },
                *nodes{id, vault_id, node_type, content, labels},
                vault_id == $vault_id
            
            :order dist
            :limit $max_results
        "#
        };

        let result = db
            .run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Vector search failed: {}", e)))?;

        if include_neighbors {
            Self::parse_search_results_with_neighbors(result.rows)
        } else {
            Self::parse_simple_search_results(result.rows)
        }
    }

    async fn export_backup(&self, vault_id: &str) -> GraphResult<GraphBackup> {
        let db = self
            .db
            .lock()
            .map_err(|e| GraphError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut params = BTreeMap::new();
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));

        let nodes_query = r#"
            ?[id, node_type, vault_id, content, labels, embedding, created_at] :=
                *nodes{
                    id,
                    node_type,
                    vault_id,
                    content,
                    labels,
                    embedding,
                    created_at
                },
                vault_id == $vault_id
            :order created_at
        "#;

        let nodes_result = db
            .run_script(nodes_query, params.clone(), ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to export nodes: {}", e)))?;

        let nodes: Vec<GraphNode> = nodes_result
            .rows
            .into_iter()
            .map(GraphNode::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let edges_query = r#"
            ?[id, from_node, to_node, edge_type, vault_id, weight, created_at] :=
                *edges{
                    id,
                    from_node,
                    to_node,
                    edge_type,
                    vault_id,
                    weight,
                    created_at
                },
                vault_id == $vault_id
            :order created_at
        "#;

        let edges_result = db
            .run_script(edges_query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to export edges: {}", e)))?;

        let edges: Vec<GraphEdge> = edges_result
            .rows
            .into_iter()
            .map(GraphEdge::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(GraphBackup {
            version: 1,
            nodes,
            edges,
            created_at: Self::get_timestamp(),
        })
    }

    async fn import_backup(&self, backup: &GraphBackup) -> GraphResult<()> {
        for node in &backup.nodes {
            self.create_node(
                &node.vault_id,
                &node.node_type,
                node.content.clone(),
                node.labels.clone(),
                node.embedding.clone(),
                Some(&node.id),
            )
            .await?;
        }

        for edge in &backup.edges {
            self.create_edge(
                &edge.vault_id,
                &edge.from_node,
                &edge.to_node,
                &edge.edge_type,
                Some(edge.weight),
                Some(&edge.id),
            )
            .await?;
        }

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use wasm_bindgen_test::*;

//     wasm_bindgen_test_configure!(run_in_browser);

//     #[wasm_bindgen_test]
//     async fn test_cozo_adapter_creation() {
//         let _adapter = CozoGraphAdapter::new();
//     }

//     #[wasm_bindgen_test]
//     async fn test_vector_search() {
//         let adapter = CozoGraphAdapter::new();

//         let emb1 = vec![1.0, 0.0, 0.0];
//         let emb2 = vec![0.9, 0.1, 0.0];
//         let emb3 = vec![0.0, 1.0, 0.0];

//         adapter
//             .create_node(
//                 "test_vault",
//                 "document",
//                 b"Doc 1".to_vec(),
//                 vec![],
//                 Some(emb1.clone()),
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         adapter
//             .create_node(
//                 "test_vault",
//                 "document",
//                 b"Doc 2".to_vec(),
//                 vec![],
//                 Some(emb2),
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         adapter
//             .create_node(
//                 "test_vault",
//                 "document",
//                 b"Doc 3".to_vec(),
//                 vec![],
//                 Some(emb3),
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         let query = vec![1.0, 0.0, 0.0];
//         let results = adapter
//             .vector_search("test_vault", query, 2, None)
//             .await
//             .unwrap();

//         assert_eq!(results.len(), 2);
//         assert_eq!(results[0].0.content, b"Doc 1");
//         assert!(results[0].1 > results[1].1);
//     }

//     #[wasm_bindgen_test]
//     async fn test_export_import_backup() {
//         let adapter1 = CozoGraphAdapter::new();

//         adapter1
//             .create_node(
//                 "test_vault",
//                 "document",
//                 b"Test".to_vec(),
//                 vec![],
//                 None,
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         let backup = adapter1.export_backup("test_vault").await.unwrap();
//         assert_eq!(backup.nodes.len(), 1);

//         let adapter2 = CozoGraphAdapter::new();
//         adapter2.import_backup(&backup).await.unwrap();

//         let nodes = adapter2
//             .list_nodes_by_type("test_vault", "document", None)
//             .await
//             .unwrap();
//         assert_eq!(nodes.len(), 1);
//     }

//     #[wasm_bindgen_test]
//     async fn test_vector_search_min_similarity() {
//         let adapter = CozoGraphAdapter::new();

//         let emb1 = vec![1.0, 0.0, 0.0];
//         let emb2 = vec![0.5, 0.5, 0.0];
//         let emb3 = vec![0.0, 1.0, 0.0];

//         adapter
//             .create_node(
//                 "test_vault",
//                 "document",
//                 b"Similar".to_vec(),
//                 vec![],
//                 Some(emb1),
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         adapter
//             .create_node(
//                 "test_vault",
//                 "document",
//                 b"Somewhat similar".to_vec(),
//                 vec![],
//                 Some(emb2),
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         adapter
//             .create_node(
//                 "test_vault",
//                 "document",
//                 b"Different".to_vec(),
//                 vec![],
//                 Some(emb3),
//                 None,
//                 None,
//             )
//             .await
//             .unwrap();

//         let query = vec![1.0, 0.0, 0.0];
//         let results = adapter
//             .vector_search("test_vault", query, 10, Some(0.7))
//             .await
//             .unwrap();

//         assert_eq!(results.len(), 2);
//         assert!(results[0].1 >= 0.7);
//         assert!(results[1].1 >= 0.7);
//     }

//     #[wasm_bindgen_test]
//     async fn test_list_nodes_with_limit() {
//         let adapter = CozoGraphAdapter::new();

//         for i in 0..5 {
//             adapter
//                 .create_node(
//                     "test_vault",
//                     "document",
//                     format!("Doc {}", i).into_bytes(),
//                     vec![],
//                     None,
//                     None,
//                     None,
//                 )
//                 .await
//                 .unwrap();
//         }

//         let all = adapter
//             .list_nodes_by_type("test_vault", "document", None)
//             .await
//             .unwrap();
//         assert_eq!(all.len(), 5);

//         let limited = adapter
//             .list_nodes_by_type("test_vault", "document", Some(3))
//             .await
//             .unwrap();
//         assert_eq!(limited.len(), 3);
//     }
// }
