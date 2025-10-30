use crate::domain::graph::{
    create_node_metadata, validate_edge, validate_node, EdgeId, EdgeProperties,
    GraphBackup, GraphEdge, GraphError, GraphNode, GraphResult, NodeId,
};
use crate::ports::graph::GraphPort;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use cozo::{DataValue, DbInstance, ScriptMutability, Vector};
use ndarray::Array1;
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

const DEFAULT_EMBEDDING_DIM: usize = 384;

static GLOBAL_COZO_DB: Lazy<Arc<Mutex<DbInstance>>> = Lazy::new(|| {
    let db = DbInstance::new("mem", "", Default::default())
        .expect("Failed to create global CozoDB instance");
    Arc::new(Mutex::new(db))
});

#[derive(Clone)]
pub struct CozoGraphAdapter {
    db: Arc<Mutex<DbInstance>>,
}

impl CozoGraphAdapter {
    pub fn new() -> Self {
        Self::try_new().expect("Failed to create CozoGraphAdapter")
    }

    pub fn try_new() -> GraphResult<Self> {
        static SCHEMA_INITIALIZED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

        let adapter = Self {
            db: GLOBAL_COZO_DB.clone(),
        };

        let mut initialized = SCHEMA_INITIALIZED.lock().unwrap();
        if !*initialized {
            adapter.init_schema()?;
            *initialized = true;
        }

        Ok(adapter)
    }

    fn init_schema(&self) -> GraphResult<()> {
        let db = self.db.lock().unwrap();

        let schema_nodes = format!(
            r#"
            :create nodes {{
                id: String =>
                node_type: String,
                vault_id: String,
                namespace: String?,
                content: String,
                labels: String,
                embedding: <F32; {}>?,
                created_at: Int,
                updated_at: Int,
                accessed_at: Int,
                access_count: Int,
            }}
            "#,
            DEFAULT_EMBEDDING_DIM
        );

        db.run_script(&schema_nodes, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to create nodes relation: {}", e)))?;

        let schema_edges = r#"
            :create edges {
                id: String =>
                from_node: String,
                to_node: String,
                edge_type: String,
                vault_id: String,
                weight: Float,
                bidirectional: Bool,
                created_at: Int,
            }
        "#;

        db.run_script(schema_edges, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to create edges relation: {}", e)))?;

        let create_hnsw_index = format!(
            r#"
            ::hnsw create nodes:embedding_idx {{
                dim: {},
                m: 50,
                dtype: F32,
                fields: [embedding],
                distance: Cosine,
                ef_construction: 200,
                extend_candidates: true,
                keep_pruned_connections: false,
            }}
            "#,
            DEFAULT_EMBEDDING_DIM
        );

        db.run_script(&create_hnsw_index, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to create HNSW index: {}", e)))?;

        Ok(())
    }

    fn get_timestamp() -> u64 {
        js_sys::Date::now() as u64
    }

    fn parse_node_from_row(row: &[DataValue]) -> GraphResult<GraphNode> {
        Self::parse_node_from_row_offset(row, 0)
    }

    fn parse_node_from_row_offset(row: &[DataValue], offset: usize) -> GraphResult<GraphNode> {
        let id = row.get(offset)
            .and_then(|v| v.get_str())
            .ok_or_else(|| GraphError::DatabaseError("Missing node id".to_string()))?;

        let node_id = NodeId::from_string(id)
            .map_err(|e| GraphError::DatabaseError(format!("Invalid node id: {}", e)))?;

        let node_type = row.get(offset + 1)
            .and_then(|v| v.get_str())
            .unwrap_or("unknown")
            .to_string();

        let vault_id = row.get(offset + 2)
            .and_then(|v| v.get_str())
            .unwrap_or("")
            .to_string();

        let namespace = row.get(offset + 3)
            .and_then(|v| v.get_str())
            .map(|s| s.to_string());

        let content_b64 = row.get(offset + 4)
            .and_then(|v| v.get_str())
            .unwrap_or("");

        let content = BASE64.decode(content_b64)
            .unwrap_or_default();

        let labels_json = row.get(offset + 5)
            .and_then(|v| v.get_str())
            .unwrap_or("[]");

        let labels: Vec<String> = serde_json::from_str(labels_json).unwrap_or_default();

        let embedding: Option<Vec<f32>> = row.get(offset + 6)
            .and_then(|v| match v {
                DataValue::Vec(Vector::F32(arr)) => Some(arr.to_vec()),
                _ => None,
            });

        let created_at = row.get(offset + 7)
            .and_then(|v| v.get_int())
            .unwrap_or(0) as u64;

        let updated_at = row.get(offset + 8)
            .and_then(|v| v.get_int())
            .unwrap_or(0) as u64;

        let accessed_at = row.get(offset + 9)
            .and_then(|v| v.get_int())
            .unwrap_or(0) as u64;

        let access_count = row.get(offset + 10)
            .and_then(|v| v.get_int())
            .unwrap_or(0) as u32;

        Ok(GraphNode {
            id: node_id,
            node_type,
            vault_id,
            namespace,
            labels,
            embedding,
            content: content.clone(),
            metadata: create_node_metadata(content.len(), None),
            created_at,
            updated_at,
            accessed_at,
            access_count,
        })
    }

    fn parse_edge_from_row(row: &[DataValue]) -> GraphResult<GraphEdge> {
        let id = row.get(0)
            .and_then(|v| v.get_str())
            .ok_or_else(|| GraphError::DatabaseError("Missing edge id".to_string()))?;

        let edge_id = EdgeId::from_string(id)
            .map_err(|e| GraphError::DatabaseError(format!("Invalid edge id: {}", e)))?;

        let from_node_str = row.get(1)
            .and_then(|v| v.get_str())
            .ok_or_else(|| GraphError::DatabaseError("Missing from_node".to_string()))?;

        let from_node = NodeId::from_string(from_node_str)
            .map_err(|e| GraphError::DatabaseError(format!("Invalid from_node: {}", e)))?;

        let to_node_str = row.get(2)
            .and_then(|v| v.get_str())
            .ok_or_else(|| GraphError::DatabaseError("Missing to_node".to_string()))?;

        let to_node = NodeId::from_string(to_node_str)
            .map_err(|e| GraphError::DatabaseError(format!("Invalid to_node: {}", e)))?;

        let edge_type = row.get(3)
            .and_then(|v| v.get_str())
            .unwrap_or("unknown")
            .to_string();

        let vault_id = row.get(4)
            .and_then(|v| v.get_str())
            .unwrap_or("")
            .to_string();

        let weight = row.get(5)
            .and_then(|v| v.get_float())
            .unwrap_or(0.5);

        let bidirectional = row.get(6)
            .and_then(|v| v.get_bool())
            .unwrap_or(false);

        let created_at = row.get(7)
            .and_then(|v| v.get_int())
            .unwrap_or(0) as u64;

        Ok(GraphEdge {
            id: edge_id,
            from_node,
            to_node,
            edge_type,
            vault_id,
            properties: EdgeProperties {
                weight,
                bidirectional,
                encrypted_context: None,
                metadata: HashMap::new(),
            },
            created_at,
        })
    }

}

impl Default for CozoGraphAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl GraphPort for CozoGraphAdapter {
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
            namespace: namespace.clone(),
            labels: labels.clone(),
            embedding: embedding.clone(),
            content: content.clone(),
            metadata: create_node_metadata(content.len(), None),
            created_at: now,
            updated_at: now,
            accessed_at: now,
            access_count: 0,
        };

        validate_node(&node)?;

        let db = self.db.lock().unwrap();

        let content_b64 = BASE64.encode(&content);
        let labels_json = serde_json::to_string(&labels).unwrap_or_else(|_| "[]".to_string());

        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(node_id.as_str().into()));
        params.insert("node_type".to_string(), DataValue::Str(node_type.into()));
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));

        if let Some(ref ns) = namespace {
            params.insert("namespace".to_string(), DataValue::Str(ns.as_str().into()));
        } else {
            params.insert("namespace".to_string(), DataValue::Null);
        }

        params.insert("content".to_string(), DataValue::Str(content_b64.as_str().into()));
        params.insert("labels".to_string(), DataValue::Str(labels_json.as_str().into()));

        if let Some(ref emb) = embedding {
            let vec_data = DataValue::Vec(Vector::F32(Array1::from_vec(emb.clone())));
            params.insert("embedding".to_string(), vec_data);
        } else {
            params.insert("embedding".to_string(), DataValue::Null);
        }

        params.insert("created_at".to_string(), DataValue::from(now as i64));
        params.insert("updated_at".to_string(), DataValue::from(now as i64));
        params.insert("accessed_at".to_string(), DataValue::from(now as i64));
        params.insert("access_count".to_string(), DataValue::from(0i64));

        let query = r#"
            ?[id, node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count] <- [[
                $id, $node_type, $vault_id, $namespace, $content, $labels, $embedding, $created_at, $updated_at, $accessed_at, $access_count
            ]]
            :put nodes { id => node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count }
        "#;

        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to insert node: {}", e)))?;

        Ok(node_id)
    }

    async fn update_node(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        content: Vec<u8>,
        embedding: Option<Vec<f32>>,
    ) -> GraphResult<()> {
        let now = Self::get_timestamp();
        let db = self.db.lock().unwrap();

        let content_b64 = BASE64.encode(&content);

        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(node_id.as_str().into()));
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert("content".to_string(), DataValue::Str(content_b64.into()));

        // Store embedding as native vector for HNSW indexing
        if let Some(ref emb) = embedding {
            let vec_data = DataValue::Vec(Vector::F32(Array1::from_vec(emb.clone())));
            params.insert("embedding".to_string(), vec_data);
        } else {
            params.insert("embedding".to_string(), DataValue::Null);
        }

        params.insert("updated_at".to_string(), DataValue::from(now as i64));

        let query = r#"
            ?[id, node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count] :=
                *nodes{
                    id,
                    node_type,
                    vault_id,
                    namespace,
                    labels,
                    created_at,
                    accessed_at,
                    access_count
                },
                id == $id,
                vault_id == $vault_id,
                content = $content,
                embedding = $embedding,
                updated_at = $updated_at
            :put nodes { id => node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count }
        "#;

        let result = db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to update node: {}", e)))?;

        if result.rows.is_empty() {
            return Err(GraphError::NodeNotFound(node_id.as_str()));
        }

        Ok(())
    }

    async fn delete_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<()> {
        let db = self.db.lock().unwrap();

        let mut params = BTreeMap::new();
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert("node_id".to_string(), DataValue::Str(node_id.as_str().into()));

        let delete_edges_query = r#"
            ?[id] :=
                *edges{
                    id,
                    vault_id,
                    from_node,
                    to_node
                },
                vault_id == $vault_id,
                (from_node == $node_id || to_node == $node_id)
            :rm edges { id }
        "#;

        db.run_script(delete_edges_query, params.clone(), ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to delete edges: {}", e)))?;

        let delete_node_query = r#"
            ?[id] :=
                *nodes{
                    id,
                    vault_id
                },
                id == $node_id,
                vault_id == $vault_id
            :rm nodes { id }
        "#;

        db.run_script(delete_node_query, params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to delete node: {}", e)))?;

        Ok(())
    }

    async fn list_nodes_by_type(
        &self,
        vault_id: &str,
        node_type: &str,
        limit: Option<usize>,
    ) -> GraphResult<Vec<GraphNode>> {
        let db = self.db.lock().unwrap();

        let mut params = BTreeMap::new();
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert("node_type".to_string(), DataValue::Str(node_type.into()));

        let limit_clause = limit.map(|l| format!(":limit {}", l)).unwrap_or_default();

        let query = format!(
            r#"
            ?[id, node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count] :=
                *nodes{{
                    id,
                    node_type,
                    vault_id,
                    namespace,
                    content,
                    labels,
                    embedding,
                    created_at,
                    updated_at,
                    accessed_at,
                    access_count
                }},
                node_type == $node_type,
                vault_id == $vault_id
            {}
            "#,
            limit_clause
        );

        let result = db.run_script(&query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to list nodes: {}", e)))?;

        let nodes: Vec<GraphNode> = result.rows.iter()
            .filter_map(|row| Self::parse_node_from_row(row).ok())
            .collect();

        Ok(nodes)
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
            properties: properties.clone(),
            created_at: now,
        };

        validate_edge(&edge)?;

        let db = self.db.lock().unwrap();

        let query = format!(
            r#"
            ?[id, from_node, to_node, edge_type, vault_id, weight, bidirectional, created_at] <- [[
                "{}", "{}", "{}", "{}", "{}", {}, {}, {}
            ]]
            :put edges {{ id => from_node, to_node, edge_type, vault_id, weight, bidirectional, created_at }}
            "#,
            edge_id.as_str(),
            from_node.as_str(),
            to_node.as_str(),
            edge_type,
            vault_id,
            properties.weight,
            properties.bidirectional,
            now
        );

        db.run_script(&query, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to insert edge: {}", e)))?;

        Ok(edge_id)
    }

    async fn get_neighbors(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        edge_types: Option<Vec<String>>,
    ) -> GraphResult<Vec<GraphNode>> {
        let db = self.db.lock().unwrap();

        let mut params = BTreeMap::new();
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert("node_id".to_string(), DataValue::Str(node_id.as_str().into()));

        // Native CozoDB query with single join - no N+1 problem!
        let query = if let Some(ref types) = edge_types {
            // With edge type filtering
            let types_json = serde_json::to_string(types).unwrap_or_else(|_| "[]".to_string());
            params.insert("edge_types".to_string(), DataValue::Str(types_json.into()));

            r#"
                ?[id, node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count] :=
                    *edges{
                        from_node,
                        to_node,
                        edge_type,
                        vault_id: edge_vault_id
                    },
                    *nodes{
                        id,
                        node_type,
                        vault_id,
                        namespace,
                        content,
                        labels,
                        embedding,
                        created_at,
                        updated_at,
                        accessed_at,
                        access_count
                    },
                    edge_vault_id == $vault_id,
                    (from_node == $node_id || to_node == $node_id),
                    (id == to_node || id == from_node),
                    id != $node_id
            "#
        } else {
            // Without filtering - all edge types
            r#"
                ?[id, node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count] :=
                    *edges{
                        from_node,
                        to_node,
                        vault_id: edge_vault_id
                    },
                    *nodes{
                        id,
                        node_type,
                        vault_id,
                        namespace,
                        content,
                        labels,
                        embedding,
                        created_at,
                        updated_at,
                        accessed_at,
                        access_count
                    },
                    edge_vault_id == $vault_id,
                    (from_node == $node_id || to_node == $node_id),
                    (id == to_node || id == from_node),
                    id != $node_id
            "#
        };

        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to get neighbors: {}", e)))?;

        let neighbors: Vec<GraphNode> = result.rows.iter()
            .filter_map(|row| Self::parse_node_from_row(row).ok())
            .collect();

        Ok(neighbors)
    }

    async fn vector_search(
        &self,
        vault_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
        min_similarity: Option<f32>,
    ) -> GraphResult<Vec<(GraphNode, f32)>> {
        let db = self.db.lock().unwrap();

        // Use HNSW index for efficient vector search
        let query_vec = DataValue::Vec(Vector::F32(Array1::from_vec(query_embedding.clone())));

        let mut params = BTreeMap::new();
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert("query_vec".to_string(), query_vec);
        params.insert("k".to_string(), DataValue::from(limit as i64));

        // HNSW query using cosine distance
        // The ~ operator performs proximity search using the HNSW index
        let hnsw_query = r#"
            ?[id, node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count, dist] :=
                ~nodes:embedding_idx{
                    id, embedding |
                    query: $query_vec,
                    k: $k,
                    ef: 100,
                    bind_distance: dist
                },
                *nodes{
                    id,
                    node_type,
                    vault_id,
                    namespace,
                    content,
                    labels,
                    embedding,
                    created_at,
                    updated_at,
                    accessed_at,
                    access_count
                },
                vault_id == $vault_id
        "#;

        let result = db.run_script(hnsw_query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("HNSW vector search failed: {}. Ensure embeddings have {} dimensions.", e, DEFAULT_EMBEDDING_DIM)))?;

        let results: Vec<(GraphNode, f32)> = result.rows.iter()
            .filter_map(|row| {
                let node = Self::parse_node_from_row(row).ok()?;

                // Distance from HNSW (cosine distance: 0 = identical, 2 = opposite)
                // Convert to similarity: similarity = 1 - (distance / 2)
                let distance = row.get(11).and_then(|v| v.get_float()).unwrap_or(2.0);
                let similarity = 1.0 - (distance as f32 / 2.0);

                if let Some(min_sim) = min_similarity {
                    if similarity < min_sim {
                        return None;
                    }
                }

                Some((node, similarity))
            })
            .collect();

        Ok(results)
    }

    async fn vector_search_with_neighbors(
        &self,
        vault_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
        min_similarity: Option<f32>,
        edge_types: Option<Vec<String>>,
    ) -> GraphResult<Vec<(GraphNode, f32, Vec<GraphNode>)>> {
        let db = self.db.lock().unwrap();

        let query_vec = DataValue::Vec(Vector::F32(Array1::from_vec(query_embedding.clone())));

        let mut params = BTreeMap::new();
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));
        params.insert("query_vec".to_string(), query_vec);
        params.insert("k".to_string(), DataValue::from(limit as i64));

        // Single native CozoDB query: HNSW + edges + neighbor nodes
        let query = if let Some(ref types) = edge_types {
            let edge_types_list = types.iter()
                .map(|t| format!("\"{}\"", t))
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                r#"
                # Step 1: HNSW vector search for relevant nodes
                found_nodes[id, dist] :=
                    ~nodes:embedding_idx{{
                        id, embedding |
                        query: $query_vec,
                        k: $k,
                        ef: 100,
                        bind_distance: dist
                    }},
                    *nodes{{ id, vault_id }},
                    vault_id == $vault_id

                # Step 2: Join with edges and neighbor nodes in single query
                ?[
                    id, node_type, vault_id, namespace, content, labels, embedding,
                    created_at, updated_at, accessed_at, access_count, dist,
                    neighbor_id, neighbor_node_type, neighbor_vault_id, neighbor_namespace,
                    neighbor_content, neighbor_labels, neighbor_embedding,
                    neighbor_created_at, neighbor_updated_at, neighbor_accessed_at, neighbor_access_count
                ] :=
                    found_nodes[id, dist],
                    *nodes{{
                        id, node_type, vault_id, namespace, content, labels, embedding,
                        created_at, updated_at, accessed_at, access_count
                    }},
                    *edges{{ from_node, to_node, edge_type, vault_id: edge_vault_id }},
                    edge_vault_id == $vault_id,
                    edge_type in [{}],
                    (from_node == id || to_node == id),
                    neighbor_id = if(from_node == id, to_node, from_node),
                    neighbor_id != id,
                    *nodes{{
                        id: neighbor_id,
                        node_type: neighbor_node_type,
                        vault_id: neighbor_vault_id,
                        namespace: neighbor_namespace,
                        content: neighbor_content,
                        labels: neighbor_labels,
                        embedding: neighbor_embedding,
                        created_at: neighbor_created_at,
                        updated_at: neighbor_updated_at,
                        accessed_at: neighbor_accessed_at,
                        access_count: neighbor_access_count
                    }}
                "#,
                edge_types_list
            )
        } else {
            r#"
                # Step 1: HNSW vector search for relevant nodes
                found_nodes[id, dist] :=
                    ~nodes:embedding_idx{
                        id, embedding |
                        query: $query_vec,
                        k: $k,
                        ef: 100,
                        bind_distance: dist
                    },
                    *nodes{ id, vault_id },
                    vault_id == $vault_id

                # Step 2: Join with edges and neighbor nodes in single query
                ?[
                    id, node_type, vault_id, namespace, content, labels, embedding,
                    created_at, updated_at, accessed_at, access_count, dist,
                    neighbor_id, neighbor_node_type, neighbor_vault_id, neighbor_namespace,
                    neighbor_content, neighbor_labels, neighbor_embedding,
                    neighbor_created_at, neighbor_updated_at, neighbor_accessed_at, neighbor_access_count
                ] :=
                    found_nodes[id, dist],
                    *nodes{
                        id, node_type, vault_id, namespace, content, labels, embedding,
                        created_at, updated_at, accessed_at, access_count
                    },
                    *edges{ from_node, to_node, vault_id: edge_vault_id },
                    edge_vault_id == $vault_id,
                    (from_node == id || to_node == id),
                    neighbor_id = if(from_node == id, to_node, from_node),
                    neighbor_id != id,
                    *nodes{
                        id: neighbor_id,
                        node_type: neighbor_node_type,
                        vault_id: neighbor_vault_id,
                        namespace: neighbor_namespace,
                        content: neighbor_content,
                        labels: neighbor_labels,
                        embedding: neighbor_embedding,
                        created_at: neighbor_created_at,
                        updated_at: neighbor_updated_at,
                        accessed_at: neighbor_accessed_at,
                        access_count: neighbor_access_count
                    }
            "#.to_string()
        };

        let result = db.run_script(&query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Vector search with neighbors failed: {}", e)))?;

        // Group results by node ID
        // Row format: [node (0-10), dist (11), neighbor (12-22)]
        use std::collections::HashMap;
        let mut nodes_map: HashMap<String, (GraphNode, f32, Vec<GraphNode>)> = HashMap::new();

        for row in result.rows.iter() {
            let node = Self::parse_node_from_row(row)?;
            let dist = row.get(11).and_then(|v| v.get_float()).unwrap_or(2.0);
            let similarity = 1.0 - (dist as f32 / 2.0);

            // Filter by min_similarity
            if let Some(min_sim) = min_similarity {
                if similarity < min_sim {
                    continue;
                }
            }

            let neighbor = Self::parse_node_from_row_offset(row, 12)?;

            let node_id = node.id.as_str();
            nodes_map.entry(node_id.clone())
                .or_insert_with(|| (node.clone(), similarity, Vec::new()))
                .2.push(neighbor);
        }

        let results: Vec<(GraphNode, f32, Vec<GraphNode>)> = nodes_map.into_values().collect();

        Ok(results)
    }

    async fn export_backup(&self, vault_id: &str) -> GraphResult<GraphBackup> {
        let db = self.db.lock().unwrap();

        let mut params = BTreeMap::new();
        params.insert("vault_id".to_string(), DataValue::Str(vault_id.into()));

        let nodes_query = r#"
            ?[id, node_type, vault_id, namespace, content, labels, embedding, created_at, updated_at, accessed_at, access_count] :=
                *nodes{
                    id,
                    node_type,
                    vault_id,
                    namespace,
                    content,
                    labels,
                    embedding,
                    created_at,
                    updated_at,
                    accessed_at,
                    access_count
                },
                vault_id == $vault_id
        "#;

        let nodes_result = db.run_script(nodes_query, params.clone(), ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to export nodes: {}", e)))?;

        let nodes: Vec<GraphNode> = nodes_result.rows.iter()
            .filter_map(|row| Self::parse_node_from_row(row).ok())
            .collect();

        let edges_query = r#"
            ?[id, from_node, to_node, edge_type, vault_id, weight, bidirectional, created_at] :=
                *edges{
                    id,
                    from_node,
                    to_node,
                    edge_type,
                    vault_id,
                    weight,
                    bidirectional,
                    created_at
                },
                vault_id == $vault_id
        "#;

        let edges_result = db.run_script(edges_query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to export edges: {}", e)))?;

        let edges: Vec<GraphEdge> = edges_result.rows.iter()
            .filter_map(|row| Self::parse_edge_from_row(row).ok())
            .collect();

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
                node.namespace.clone(),
            ).await?;
        }

        for edge in &backup.edges {
            self.create_edge(
                &edge.vault_id,
                &edge.from_node,
                &edge.to_node,
                &edge.edge_type,
                edge.properties.clone(),
            ).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_cozo_adapter_creation() {
        let _adapter = CozoGraphAdapter::new();
    }

    #[wasm_bindgen_test]
    async fn test_vector_search() {
        let adapter = CozoGraphAdapter::new();

        let emb1 = vec![1.0, 0.0, 0.0];
        let emb2 = vec![0.9, 0.1, 0.0];
        let emb3 = vec![0.0, 1.0, 0.0];

        adapter.create_node(
            "test_vault",
            "document",
            b"Doc 1".to_vec(),
            vec![],
            Some(emb1.clone()),
            None,
        ).await.unwrap();

        adapter.create_node(
            "test_vault",
            "document",
            b"Doc 2".to_vec(),
            vec![],
            Some(emb2),
            None,
        ).await.unwrap();

        adapter.create_node(
            "test_vault",
            "document",
            b"Doc 3".to_vec(),
            vec![],
            Some(emb3),
            None,
        ).await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = adapter.vector_search("test_vault", query, 2, None).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.content, b"Doc 1");
        assert!(results[0].1 > results[1].1);
    }

    #[wasm_bindgen_test]
    async fn test_export_import_backup() {
        let adapter1 = CozoGraphAdapter::new();

        adapter1.create_node(
            "test_vault",
            "document",
            b"Test".to_vec(),
            vec![],
            None,
            None,
        ).await.unwrap();

        let backup = adapter1.export_backup("test_vault").await.unwrap();
        assert_eq!(backup.nodes.len(), 1);

        let adapter2 = CozoGraphAdapter::new();
        adapter2.import_backup(&backup).await.unwrap();

        let nodes = adapter2.list_nodes_by_type("test_vault", "document", None).await.unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[wasm_bindgen_test]
    async fn test_get_neighbors() {
        let adapter = CozoGraphAdapter::new();

        let node1 = adapter.create_node(
            "test_vault",
            "document",
            b"Node 1".to_vec(),
            vec![],
            None,
            None,
        ).await.unwrap();

        let node2 = adapter.create_node(
            "test_vault",
            "document",
            b"Node 2".to_vec(),
            vec![],
            None,
            None,
        ).await.unwrap();

        let node3 = adapter.create_node(
            "test_vault",
            "document",
            b"Node 3".to_vec(),
            vec![],
            None,
            None,
        ).await.unwrap();

        let props = EdgeProperties {
            weight: 1.0,
            bidirectional: false,
            encrypted_context: None,
            metadata: HashMap::new(),
        };

        adapter.create_edge("test_vault", &node1, &node2, "references", props.clone()).await.unwrap();
        adapter.create_edge("test_vault", &node1, &node3, "cites", props).await.unwrap();

        let all_neighbors = adapter.get_neighbors("test_vault", &node1, None).await.unwrap();
        assert_eq!(all_neighbors.len(), 2);

        let filtered = adapter.get_neighbors(
            "test_vault",
            &node1,
            Some(vec!["references".to_string()]),
        ).await.unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].content, b"Node 2");
    }

    #[wasm_bindgen_test]
    async fn test_vector_search_min_similarity() {
        let adapter = CozoGraphAdapter::new();

        let emb1 = vec![1.0, 0.0, 0.0];
        let emb2 = vec![0.5, 0.5, 0.0];
        let emb3 = vec![0.0, 1.0, 0.0];

        adapter.create_node(
            "test_vault",
            "document",
            b"Similar".to_vec(),
            vec![],
            Some(emb1),
            None,
        ).await.unwrap();

        adapter.create_node(
            "test_vault",
            "document",
            b"Somewhat similar".to_vec(),
            vec![],
            Some(emb2),
            None,
        ).await.unwrap();

        adapter.create_node(
            "test_vault",
            "document",
            b"Different".to_vec(),
            vec![],
            Some(emb3),
            None,
        ).await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = adapter.vector_search("test_vault", query, 10, Some(0.7)).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].1 >= 0.7);
        assert!(results[1].1 >= 0.7);
    }

    #[wasm_bindgen_test]
    async fn test_list_nodes_with_limit() {
        let adapter = CozoGraphAdapter::new();

        for i in 0..5 {
            adapter.create_node(
                "test_vault",
                "document",
                format!("Doc {}", i).into_bytes(),
                vec![],
                None,
                None,
            ).await.unwrap();
        }

        let all = adapter.list_nodes_by_type("test_vault", "document", None).await.unwrap();
        assert_eq!(all.len(), 5);

        let limited = adapter.list_nodes_by_type("test_vault", "document", Some(3)).await.unwrap();
        assert_eq!(limited.len(), 3);
    }

}
