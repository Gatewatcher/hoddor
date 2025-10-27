use super::error::{GraphError, GraphResult};
use super::types::{EdgeProperties, GraphEdge, GraphNode, NodeMetadata};

pub fn validate_node(node: &GraphNode) -> GraphResult<()> {
    if node.vault_id.is_empty() {
        return Err(GraphError::Other("vault_id cannot be empty".to_string()));
    }

    if node.node_type.is_empty() {
        return Err(GraphError::InvalidNodeType(
            "node_type cannot be empty".to_string(),
        ));
    }

    if node.content.is_empty() {
        return Err(GraphError::Other("content cannot be empty".to_string()));
    }

    if let Some(ref emb) = node.embedding {
        if emb.is_empty() {
            return Err(GraphError::InvalidEmbedding(
                "embedding cannot be empty if present".to_string(),
            ));
        }
    }

    Ok(())
}

pub fn validate_edge(edge: &GraphEdge) -> GraphResult<()> {
    if edge.vault_id.is_empty() {
        return Err(GraphError::Other("vault_id cannot be empty".to_string()));
    }

    if edge.edge_type.is_empty() {
        return Err(GraphError::InvalidEdgeType(
            "edge_type cannot be empty".to_string(),
        ));
    }

    if edge.properties.weight < 0.0 || edge.properties.weight > 1.0 {
        return Err(GraphError::Other(
            "edge weight must be between 0.0 and 1.0".to_string(),
        ));
    }

    if edge.from_node == edge.to_node {
        return Err(GraphError::Other("self-loops are not allowed".to_string()));
    }

    Ok(())
}

pub fn create_node_metadata(content_size: usize, expires_at: Option<u64>) -> NodeMetadata {
    NodeMetadata {
        content_size,
        version: 1,
        expires_at,
    }
}

pub fn create_edge_properties(weight: f64, bidirectional: bool) -> EdgeProperties {
    EdgeProperties {
        weight,
        bidirectional,
        encrypted_context: None,
        metadata: Default::default(),
    }
}

pub fn is_node_expired(node: &GraphNode, current_time: u64) -> bool {
    if let Some(expires_at) = node.metadata.expires_at {
        current_time > expires_at
    } else {
        false
    }
}

pub fn update_node_access(node: &mut GraphNode, current_time: u64) {
    node.accessed_at = current_time;
    node.access_count = node.access_count.saturating_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::graph::types::{EdgeId, NodeId};

    #[test]
    fn test_validate_node_valid() {
        let node = GraphNode {
            id: NodeId::new(),
            node_type: "memory".to_string(),
            vault_id: "test_vault".to_string(),
            namespace: None,
            labels: vec![],
            embedding: None,
            content: vec![1, 2, 3],
            metadata: NodeMetadata {
                content_size: 3,
                version: 1,
                expires_at: None,
            },
            created_at: 0,
            updated_at: 0,
            accessed_at: 0,
            access_count: 0,
        };

        assert!(validate_node(&node).is_ok());
    }

    #[test]
    fn test_validate_node_empty_vault() {
        let mut node = GraphNode {
            id: NodeId::new(),
            node_type: "memory".to_string(),
            vault_id: "".to_string(),
            namespace: None,
            labels: vec![],
            embedding: None,
            content: vec![1, 2, 3],
            metadata: NodeMetadata {
                content_size: 3,
                version: 1,
                expires_at: None,
            },
            created_at: 0,
            updated_at: 0,
            accessed_at: 0,
            access_count: 0,
        };

        node.vault_id = "".to_string();
        assert!(validate_node(&node).is_err());
    }

    #[test]
    fn test_validate_edge_valid() {
        let edge = GraphEdge {
            id: EdgeId::new(),
            from_node: NodeId::new(),
            to_node: NodeId::new(),
            edge_type: "relates_to".to_string(),
            vault_id: "test_vault".to_string(),
            properties: EdgeProperties {
                weight: 0.8,
                bidirectional: false,
                encrypted_context: None,
                metadata: Default::default(),
            },
            created_at: 0,
        };

        assert!(validate_edge(&edge).is_ok());
    }

    #[test]
    fn test_validate_edge_self_loop() {
        let node_id = NodeId::new();
        let edge = GraphEdge {
            id: EdgeId::new(),
            from_node: node_id.clone(),
            to_node: node_id,
            edge_type: "relates_to".to_string(),
            vault_id: "test_vault".to_string(),
            properties: EdgeProperties::default(),
            created_at: 0,
        };

        assert!(validate_edge(&edge).is_err());
    }

    #[test]
    fn test_is_node_expired() {
        let mut node = GraphNode {
            id: NodeId::new(),
            node_type: "memory".to_string(),
            vault_id: "test_vault".to_string(),
            namespace: None,
            labels: vec![],
            embedding: None,
            content: vec![1, 2, 3],
            metadata: NodeMetadata {
                content_size: 3,
                version: 1,
                expires_at: Some(100),
            },
            created_at: 0,
            updated_at: 0,
            accessed_at: 0,
            access_count: 0,
        };

        assert!(!is_node_expired(&node, 50));
        assert!(is_node_expired(&node, 150));

        node.metadata.expires_at = None;
        assert!(!is_node_expired(&node, 1000));
    }
}
