use crate::domain::graph::{
    create_node_metadata, validate_edge, validate_node, EdgeDirection, EdgeId, EdgeProperties,
    GraphEdge, GraphError, GraphNode, GraphResult, NodeId,
};
use crate::ports::graph::GraphPort;
use async_trait::async_trait;
use cozo::{DataValue, DbInstance, ScriptMutability};
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct CozoGraphAdapter {
    db: Arc<DbInstance>,
}

impl CozoGraphAdapter {
    pub fn new_in_memory() -> GraphResult<Self> {
        let db = DbInstance::new("mem", "", Default::default())
            .map_err(|e| GraphError::DatabaseError(format!("Failed to create database: {}", e)))?;

        let adapter = Self { db: Arc::new(db) };

        adapter.initialize_schema()?;

        Ok(adapter)
    }

    fn json_to_datavalue(value: &serde_json::Value) -> DataValue {
        match value {
            serde_json::Value::Null => DataValue::Null,
            serde_json::Value::Bool(b) => DataValue::from(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    DataValue::from(i)
                } else if let Some(f) = n.as_f64() {
                    DataValue::from(f)
                } else {
                    DataValue::Null
                }
            }
            serde_json::Value::String(s) => DataValue::from(s.as_str()),
            serde_json::Value::Array(arr) => {
                let vec: Vec<DataValue> = arr.iter().map(|v| Self::json_to_datavalue(v)).collect();
                DataValue::List(vec)
            }
            serde_json::Value::Object(_) => DataValue::from(value.to_string()),
        }
    }

    fn build_params(json: serde_json::Value) -> BTreeMap<String, DataValue> {
        let mut params = BTreeMap::new();
        if let serde_json::Value::Object(map) = json {
            for (k, v) in map {
                params.insert(k, Self::json_to_datavalue(&v));
            }
        }
        params
    }

    fn initialize_schema(&self) -> GraphResult<()> {
        let nodes_schema = r#"
            :create nodes {
                id: String
                =>
                node_type: String,
                vault_id: String,
                namespace: String?,
                labels: [String],
                embedding: [Float]?,
                encrypted_content: Bytes,
                content_hmac: String,
                content_size: Int,
                version: Int,
                expires_at: Int?,
                created_at: Int,
                updated_at: Int,
                accessed_at: Int,
                access_count: Int
            }
        "#;

        self.db
            .run_script(nodes_schema, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| {
                GraphError::DatabaseError(format!("Failed to create nodes schema: {}", e))
            })?;

        let edges_schema = r#"
            :create edges {
                id: String
                =>
                from_node: String,
                to_node: String,
                edge_type: String,
                vault_id: String,
                weight: Float,
                bidirectional: Bool,
                encrypted_context: Bytes?,
                created_at: Int
            }
        "#;

        self.db
            .run_script(edges_schema, Default::default(), ScriptMutability::Mutable)
            .map_err(|e| {
                GraphError::DatabaseError(format!("Failed to create edges schema: {}", e))
            })?;

        Ok(())
    }

    fn deserialize_node(&self, row: &[serde_json::Value]) -> GraphResult<GraphNode> {
        if row.len() < 15 {
            return Err(GraphError::SerializationError(
                "Invalid row length".to_string(),
            ));
        }

        let id = NodeId::from_string(
            row[0]
                .as_str()
                .ok_or_else(|| GraphError::SerializationError("Invalid id".to_string()))?,
        )
        .map_err(|e| GraphError::SerializationError(format!("Invalid UUID: {}", e)))?;

        let node_type = row[1]
            .as_str()
            .ok_or_else(|| GraphError::SerializationError("Invalid node_type".to_string()))?
            .to_string();

        let vault_id = row[2]
            .as_str()
            .ok_or_else(|| GraphError::SerializationError("Invalid vault_id".to_string()))?
            .to_string();

        let namespace = row[3].as_str().map(|s| s.to_string());

        let labels: Vec<String> = row[4]
            .as_array()
            .ok_or_else(|| GraphError::SerializationError("Invalid labels".to_string()))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let embedding: Option<Vec<f32>> = row[5].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect()
        });

        let encrypted_content: Vec<u8> = row[6]
            .as_array()
            .ok_or_else(|| GraphError::SerializationError("Invalid encrypted_content".to_string()))?
            .iter()
            .filter_map(|v| v.as_u64().map(|u| u as u8))
            .collect();

        let content_hmac = row[7]
            .as_str()
            .ok_or_else(|| GraphError::SerializationError("Invalid content_hmac".to_string()))?
            .to_string();

        let content_size = row[8]
            .as_u64()
            .ok_or_else(|| GraphError::SerializationError("Invalid content_size".to_string()))?
            as usize;

        let _version = row[9]
            .as_u64()
            .ok_or_else(|| GraphError::SerializationError("Invalid version".to_string()))?
            as u32;

        let expires_at = row[10].as_u64();

        let created_at = row[11]
            .as_u64()
            .ok_or_else(|| GraphError::SerializationError("Invalid created_at".to_string()))?;

        let updated_at = row[12]
            .as_u64()
            .ok_or_else(|| GraphError::SerializationError("Invalid updated_at".to_string()))?;

        let accessed_at = row[13]
            .as_u64()
            .ok_or_else(|| GraphError::SerializationError("Invalid accessed_at".to_string()))?;

        let access_count = row[14]
            .as_u64()
            .ok_or_else(|| GraphError::SerializationError("Invalid access_count".to_string()))?
            as u32;

        Ok(GraphNode {
            id,
            node_type,
            vault_id,
            namespace,
            labels,
            embedding,
            encrypted_content,
            content_hmac,
            metadata: create_node_metadata(content_size, expires_at),
            created_at,
            updated_at,
            accessed_at,
            access_count,
        })
    }

    fn deserialize_edge(&self, row: &[serde_json::Value]) -> GraphResult<GraphEdge> {
        if row.len() < 9 {
            return Err(GraphError::SerializationError(
                "Invalid row length".to_string(),
            ));
        }

        let id = EdgeId::from_string(
            row[0]
                .as_str()
                .ok_or_else(|| GraphError::SerializationError("Invalid id".to_string()))?,
        )
        .map_err(|e| GraphError::SerializationError(format!("Invalid UUID: {}", e)))?;

        let from_node = NodeId::from_string(
            row[1]
                .as_str()
                .ok_or_else(|| GraphError::SerializationError("Invalid from_node".to_string()))?,
        )
        .map_err(|e| GraphError::SerializationError(format!("Invalid UUID: {}", e)))?;

        let to_node = NodeId::from_string(
            row[2]
                .as_str()
                .ok_or_else(|| GraphError::SerializationError("Invalid to_node".to_string()))?,
        )
        .map_err(|e| GraphError::SerializationError(format!("Invalid UUID: {}", e)))?;

        let edge_type = row[3]
            .as_str()
            .ok_or_else(|| GraphError::SerializationError("Invalid edge_type".to_string()))?
            .to_string();

        let vault_id = row[4]
            .as_str()
            .ok_or_else(|| GraphError::SerializationError("Invalid vault_id".to_string()))?
            .to_string();

        let weight = row[5]
            .as_f64()
            .ok_or_else(|| GraphError::SerializationError("Invalid weight".to_string()))?;

        let bidirectional = row[6]
            .as_bool()
            .ok_or_else(|| GraphError::SerializationError("Invalid bidirectional".to_string()))?;

        let encrypted_context: Option<Vec<u8>> = row[7].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64().map(|u| u as u8))
                .collect()
        });

        let created_at = row[8]
            .as_u64()
            .ok_or_else(|| GraphError::SerializationError("Invalid created_at".to_string()))?;

        Ok(GraphEdge {
            id,
            from_node,
            to_node,
            edge_type,
            vault_id,
            properties: EdgeProperties {
                weight,
                bidirectional,
                encrypted_context,
                metadata: Default::default(),
            },
            created_at,
        })
    }

    fn get_timestamp() -> u64 {
        js_sys::Date::now() as u64
    }
}

#[async_trait(?Send)]
impl GraphPort for CozoGraphAdapter {
    async fn create_node(
        &self,
        vault_id: &str,
        node_type: &str,
        encrypted_content: Vec<u8>,
        content_hmac: String,
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
            encrypted_content,
            content_hmac,
            metadata: create_node_metadata(0, None),
            created_at: now,
            updated_at: now,
            accessed_at: now,
            access_count: 0,
        };

        validate_node(&node)?;

        let query = r#"
            ?[id, node_type, vault_id, namespace, labels, embedding, encrypted_content,
              content_hmac, content_size, version, expires_at, created_at, updated_at,
              accessed_at, access_count] <- [[$id, $node_type, $vault_id, $namespace,
              $labels, $embedding, $encrypted_content, $content_hmac, $content_size,
              $version, $expires_at, $created_at, $updated_at, $accessed_at, $access_count]]

            :put nodes {
                id => node_type, vault_id, namespace, labels, embedding, encrypted_content,
                content_hmac, content_size, version, expires_at, created_at, updated_at,
                accessed_at, access_count
            }
        "#;

        let mut params = BTreeMap::new();
        params.insert(
            "node_type".to_string(),
            DataValue::from(node.node_type.as_str()),
        );
        params.insert(
            "vault_id".to_string(),
            DataValue::from(node.vault_id.as_str()),
        );
        params.insert(
            "namespace".to_string(),
            node.namespace
                .as_ref()
                .map(|s| DataValue::from(s.as_str()))
                .unwrap_or(DataValue::Null),
        );
        params.insert(
            "labels".to_string(),
            DataValue::List(
                node.labels
                    .iter()
                    .map(|s| DataValue::from(s.as_str()))
                    .collect(),
            ),
        );
        params.insert(
            "embedding".to_string(),
            node.embedding
                .as_ref()
                .map(|v| DataValue::List(v.iter().map(|f| DataValue::from(*f as f64)).collect()))
                .unwrap_or(DataValue::Null),
        );
        params.insert(
            "encrypted_content".to_string(),
            DataValue::Bytes(node.encrypted_content.clone()),
        );
        params.insert(
            "content_hmac".to_string(),
            DataValue::from(node.content_hmac.as_str()),
        );
        params.insert(
            "content_size".to_string(),
            DataValue::from(node.metadata.content_size as i64),
        );
        params.insert(
            "version".to_string(),
            DataValue::from(node.metadata.version as i64),
        );
        params.insert(
            "expires_at".to_string(),
            node.metadata
                .expires_at
                .map(|e| DataValue::from(e as i64))
                .unwrap_or(DataValue::Null),
        );
        params.insert(
            "created_at".to_string(),
            DataValue::from(node.created_at as i64),
        );
        params.insert(
            "updated_at".to_string(),
            DataValue::from(node.updated_at as i64),
        );
        params.insert(
            "accessed_at".to_string(),
            DataValue::from(node.accessed_at as i64),
        );
        params.insert(
            "access_count".to_string(),
            DataValue::from(node.access_count as i64),
        );
        params.insert("id".to_string(), DataValue::from(node.id.as_str()));

        self.db
            .run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to insert node: {}", e)))?;

        Ok(node_id)
    }

    async fn get_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<Option<GraphNode>> {
        let query = r#"
            ?[id, node_type, vault_id, namespace, labels, embedding, encrypted_content,
              content_hmac, content_size, version, expires_at, created_at, updated_at,
              accessed_at, access_count] :=
                *nodes{
                    id, node_type, vault_id, namespace, labels, embedding, encrypted_content,
                    content_hmac, content_size, version, expires_at, created_at, updated_at,
                    accessed_at, access_count
                },
                id == $id,
                vault_id == $vault_id
        "#;

        let params_json = serde_json::json!({
            "id": node_id.as_str(),
            "vault_id": vault_id,
        });
        let params = Self::build_params(params_json);

        let result = self
            .db
            .run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to query node: {}", e)))?;

        let result_json: serde_json::Value = serde_json::to_value(&result).map_err(|e| {
            GraphError::SerializationError(format!("Failed to serialize result: {}", e))
        })?;

        let rows = result_json["rows"]
            .as_array()
            .ok_or_else(|| GraphError::DatabaseError("Invalid result format".to_string()))?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = rows[0]
            .as_array()
            .ok_or_else(|| GraphError::DatabaseError("Invalid row format".to_string()))?;

        let node = self.deserialize_node(row)?;
        Ok(Some(node))
    }

    async fn update_node(
        &self,
        _vault_id: &str,
        node_id: &NodeId,
        encrypted_content: Vec<u8>,
        content_hmac: String,
        embedding: Option<Vec<f32>>,
    ) -> GraphResult<()> {
        let now = Self::get_timestamp();

        let query = r#"
            ?[id, encrypted_content, content_hmac, embedding, updated_at] <-
                [[$id, $encrypted_content, $content_hmac, $embedding, $updated_at]]

            :update nodes {
                id => encrypted_content, content_hmac, embedding, updated_at
            }
        "#;

        let params_json = serde_json::json!({
            "id": node_id.as_str(),
            "encrypted_content": encrypted_content,
            "content_hmac": content_hmac,
            "embedding": embedding,
            "updated_at": now,
        });
        let params = Self::build_params(params_json);

        self.db
            .run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to update node: {}", e)))?;

        Ok(())
    }

    async fn delete_node(&self, vault_id: &str, node_id: &NodeId) -> GraphResult<()> {
        let delete_edges = r#"
            ?[id] := *edges{id, from_node: from, to_node: to, vault_id: vid},
                     (from == $node_id || to == $node_id),
                     vid == $vault_id

            :rm edges {id}
        "#;

        let edge_params_json = serde_json::json!({
            "node_id": node_id.as_str(),
            "vault_id": vault_id,
        });
        let edge_params = Self::build_params(edge_params_json);

        self.db
            .run_script(delete_edges, edge_params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to delete edges: {}", e)))?;

        let delete_node = r#"
            ?[id] := *nodes{id, vault_id: vid},
                     id == $node_id,
                     vid == $vault_id

            :rm nodes {id}
        "#;

        let node_params_json = serde_json::json!({
            "node_id": node_id.as_str(),
            "vault_id": vault_id,
        });
        let node_params = Self::build_params(node_params_json);

        self.db
            .run_script(delete_node, node_params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to delete node: {}", e)))?;

        Ok(())
    }

    async fn list_nodes_by_type(
        &self,
        _vault_id: &str,
        _node_type: &str,
        _limit: Option<usize>,
    ) -> GraphResult<Vec<GraphNode>> {
        Ok(vec![])
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

        let query = r#"
            ?[id, from_node, to_node, edge_type, vault_id, weight, bidirectional,
              encrypted_context, created_at] <- [[$id, $from_node, $to_node, $edge_type,
              $vault_id, $weight, $bidirectional, $encrypted_context, $created_at]]

            :put edges {
                id => from_node, to_node, edge_type, vault_id, weight, bidirectional,
                encrypted_context, created_at
            }
        "#;

        let params_json = serde_json::json!({
            "id": edge.id.as_str(),
            "from_node": edge.from_node.as_str(),
            "to_node": edge.to_node.as_str(),
            "edge_type": edge.edge_type,
            "vault_id": edge.vault_id,
            "weight": edge.properties.weight,
            "bidirectional": edge.properties.bidirectional,
            "encrypted_context": edge.properties.encrypted_context,
            "created_at": edge.created_at,
        });
        let params = Self::build_params(params_json);

        self.db
            .run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to insert edge: {}", e)))?;

        Ok(edge_id)
    }

    async fn get_edges(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        direction: EdgeDirection,
    ) -> GraphResult<Vec<GraphEdge>> {
        let query = match direction {
            EdgeDirection::Outgoing => {
                r#"
                ?[id, from_node, to_node, edge_type, vault_id, weight, bidirectional,
                  encrypted_context, created_at] :=
                    *edges{id, from_node, to_node, edge_type, vault_id, weight, bidirectional,
                           encrypted_context, created_at},
                    from_node == $node_id,
                    vault_id == $vault_id
            "#
            }
            EdgeDirection::Incoming => {
                r#"
                ?[id, from_node, to_node, edge_type, vault_id, weight, bidirectional,
                  encrypted_context, created_at] :=
                    *edges{id, from_node, to_node, edge_type, vault_id, weight, bidirectional,
                           encrypted_context, created_at},
                    to_node == $node_id,
                    vault_id == $vault_id
            "#
            }
            EdgeDirection::Both => {
                r#"
                ?[id, from_node, to_node, edge_type, vault_id, weight, bidirectional,
                  encrypted_context, created_at] :=
                    *edges{id, from_node, to_node, edge_type, vault_id, weight, bidirectional,
                           encrypted_context, created_at},
                    (from_node == $node_id || to_node == $node_id),
                    vault_id == $vault_id
            "#
            }
        };

        let params_json = serde_json::json!({
            "node_id": node_id.as_str(),
            "vault_id": vault_id,
        });
        let params = Self::build_params(params_json);

        let result = self
            .db
            .run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to query edges: {}", e)))?;

        let result_json: serde_json::Value = serde_json::to_value(&result).map_err(|e| {
            GraphError::SerializationError(format!("Failed to serialize result: {}", e))
        })?;

        let rows = result_json["rows"]
            .as_array()
            .ok_or_else(|| GraphError::DatabaseError("Invalid result format".to_string()))?;

        let mut edges = Vec::new();
        for row_value in rows {
            let row = row_value
                .as_array()
                .ok_or_else(|| GraphError::DatabaseError("Invalid row format".to_string()))?;

            let edge = self.deserialize_edge(row)?;
            edges.push(edge);
        }

        Ok(edges)
    }

    async fn delete_edge(&self, _vault_id: &str, _edge_id: &EdgeId) -> GraphResult<()> {
        Err(GraphError::Other(
            "Delete edge not yet implemented".to_string(),
        ))
    }

    async fn get_neighbors(
        &self,
        vault_id: &str,
        node_id: &NodeId,
        edge_types: Option<Vec<String>>,
    ) -> GraphResult<Vec<GraphNode>> {
        let query = if let Some(types) = edge_types {
            let types_list = types
                .iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                r#"
                ?[neighbor_id] :=
                    *edges{{from_node: from, to_node: to, edge_type: etype, vault_id: vid}},
                    (from == $node_id || to == $node_id),
                    vid == $vault_id,
                    etype in [{}],
                    neighbor_id = if(from == $node_id, to, from)

                ?[id, node_type, vault_id, namespace, labels, embedding, encrypted_content,
                  content_hmac, content_size, version, expires_at, created_at, updated_at,
                  accessed_at, access_count] :=
                    *neighbor_id{{neighbor_id: id}},
                    *nodes{{id, node_type, vault_id, namespace, labels, embedding, encrypted_content,
                           content_hmac, content_size, version, expires_at, created_at, updated_at,
                           accessed_at, access_count}}
            "#,
                types_list
            )
        } else {
            r#"
                ?[neighbor_id] :=
                    *edges{from_node: from, to_node: to, vault_id: vid},
                    (from == $node_id || to == $node_id),
                    vid == $vault_id,
                    neighbor_id = if(from == $node_id, to, from)

                ?[id, node_type, vault_id, namespace, labels, embedding, encrypted_content,
                  content_hmac, content_size, version, expires_at, created_at, updated_at,
                  accessed_at, access_count] :=
                    *neighbor_id{neighbor_id: id},
                    *nodes{id, node_type, vault_id, namespace, labels, embedding, encrypted_content,
                           content_hmac, content_size, version, expires_at, created_at, updated_at,
                           accessed_at, access_count}
            "#
            .to_string()
        };

        let params_json = serde_json::json!({
            "node_id": node_id.as_str(),
            "vault_id": vault_id,
        });
        let params = Self::build_params(params_json);

        let result = self
            .db
            .run_script(&query, params, ScriptMutability::Immutable)
            .map_err(|e| GraphError::DatabaseError(format!("Failed to query neighbors: {}", e)))?;

        let result_json: serde_json::Value = serde_json::to_value(&result).map_err(|e| {
            GraphError::SerializationError(format!("Failed to parse result: {}", e))
        })?;

        let rows = result_json["rows"]
            .as_array()
            .ok_or_else(|| GraphError::DatabaseError("Invalid result format".to_string()))?;

        let mut nodes = Vec::new();
        for row_value in rows {
            let row = row_value
                .as_array()
                .ok_or_else(|| GraphError::DatabaseError("Invalid row format".to_string()))?;

            let node = self.deserialize_node(row)?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    async fn vector_search(
        &self,
        vault_id: &str,
        query_embedding: Vec<f32>,
        limit: usize,
        min_similarity: Option<f32>,
    ) -> GraphResult<Vec<(GraphNode, f32)>> {
        let _ = (vault_id, query_embedding, limit, min_similarity);
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::CozoGraphAdapter;
    use crate::domain::graph::{EdgeDirection, EdgeProperties, EdgeType, NodeId};
    use crate::ports::graph::GraphPort;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_create_graph_adapter() {
        let adapter = CozoGraphAdapter::new_in_memory();
        assert!(adapter.is_ok());
    }

    #[wasm_bindgen_test]
    async fn test_create_and_get_node() {
        let adapter = CozoGraphAdapter::new_in_memory().unwrap();

        let content = vec![1, 2, 3, 4, 5];
        let node_id = adapter
            .create_node(
                "test_vault",
                "memory",
                content.clone(),
                "test_hmac_123".to_string(),
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
        assert_eq!(node.encrypted_content, content);
        assert_eq!(node.content_hmac, "test_hmac_123");
        assert!(node.embedding.is_some());
    }

    #[wasm_bindgen_test]
    async fn test_create_update_get_node() {
        let adapter = CozoGraphAdapter::new_in_memory().unwrap();

        let node_id = adapter
            .create_node(
                "test_vault",
                "memory",
                vec![1, 2, 3],
                "hmac1".to_string(),
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
                "hmac2".to_string(),
                Some(vec![0.5, 0.6]),
            )
            .await
            .expect("Failed to update node");

        let node = adapter
            .get_node("test_vault", &node_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(node.encrypted_content, new_content);
        assert_eq!(node.content_hmac, "hmac2");
        assert_eq!(node.embedding, Some(vec![0.5, 0.6]));
    }

    #[wasm_bindgen_test]
    async fn test_create_and_delete_node() {
        let adapter = CozoGraphAdapter::new_in_memory().unwrap();

        let node_id = adapter
            .create_node(
                "test_vault",
                "memory",
                vec![1, 2, 3],
                "hmac".to_string(),
                vec![],
                None,
                None,
            )
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
        let adapter = CozoGraphAdapter::new_in_memory().unwrap();

        let from_id = adapter
            .create_node(
                "test_vault",
                "entity",
                vec![1],
                "hmac1".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        let to_id = adapter
            .create_node(
                "test_vault",
                "entity",
                vec![2],
                "hmac2".to_string(),
                vec![],
                None,
                None,
            )
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
        let adapter = CozoGraphAdapter::new_in_memory().unwrap();

        let node_a = adapter
            .create_node(
                "test_vault",
                "entity",
                vec![1],
                "h1".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        let node_b = adapter
            .create_node(
                "test_vault",
                "entity",
                vec![2],
                "h2".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        let node_c = adapter
            .create_node(
                "test_vault",
                "entity",
                vec![3],
                "h3".to_string(),
                vec![],
                None,
                None,
            )
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
        let adapter = CozoGraphAdapter::new_in_memory().unwrap();

        let node_a = adapter
            .create_node(
                "test_vault",
                "entity",
                vec![1],
                "h1".to_string(),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        let node_b = adapter
            .create_node(
                "test_vault",
                "entity",
                vec![2],
                "h2".to_string(),
                vec![],
                None,
                None,
            )
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
