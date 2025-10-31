use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(pub Uuid);

impl Id {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: Id,
    pub node_type: String,
    pub vault_id: String,
    pub content: String,
    pub labels: Vec<String>,
    pub embedding: Option<Vec<f32>>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: Id,
    pub from_node: Id,
    pub to_node: Id,
    pub edge_type: String,
    pub vault_id: String,
    pub weight: f32,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub node: GraphNode,
    pub distance: f32,
    pub neighbors: Vec<NeighborNode>,
}

#[derive(Debug, Clone)]
pub struct NeighborNode {
    pub node: GraphNode,
    pub edge_type: String,
    pub weight: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphBackup {
    pub version: u32,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub created_at: u64,
}
