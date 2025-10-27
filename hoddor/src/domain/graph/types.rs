use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
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

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);

impl EdgeId {
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

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,

    pub node_type: String,

    pub vault_id: String,

    pub namespace: Option<String>,

    pub labels: Vec<String>,

    pub embedding: Option<Vec<f32>>,

    pub content: Vec<u8>,

    pub metadata: NodeMetadata,

    pub created_at: u64,
    pub updated_at: u64,
    pub accessed_at: u64,

    pub access_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub content_size: usize,

    pub version: u32,

    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: EdgeId,

    pub from_node: NodeId,

    pub to_node: NodeId,

    pub edge_type: String,

    pub vault_id: String,

    pub properties: EdgeProperties,

    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeProperties {
    pub weight: f64,

    pub bidirectional: bool,

    pub encrypted_context: Option<Vec<u8>>,

    pub metadata: HashMap<String, String>,
}

impl Default for EdgeProperties {
    fn default() -> Self {
        Self {
            weight: 1.0,
            bidirectional: false,
            encrypted_context: None,
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeDirection {
    Incoming,
    Outgoing,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Memory,
    Entity,
    Event,
    Concept,
    Conversation,
    Document,
    Preference,
    Custom(String),
}

impl NodeType {
    pub fn as_str(&self) -> &str {
        match self {
            NodeType::Memory => "memory",
            NodeType::Entity => "entity",
            NodeType::Event => "event",
            NodeType::Concept => "concept",
            NodeType::Conversation => "conversation",
            NodeType::Document => "document",
            NodeType::Preference => "preference",
            NodeType::Custom(s) => s,
        }
    }
}

impl From<String> for NodeType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "memory" => NodeType::Memory,
            "entity" => NodeType::Entity,
            "event" => NodeType::Event,
            "concept" => NodeType::Concept,
            "conversation" => NodeType::Conversation,
            "document" => NodeType::Document,
            "preference" => NodeType::Preference,
            _ => NodeType::Custom(s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeType {
    RelatesTo,
    IsA,
    PartOf,
    LocatedIn,
    HappenedAt,
    CausedBy,
    SimilarTo,
    References,
    Prefers,
    Custom(String),
}

impl EdgeType {
    pub fn as_str(&self) -> &str {
        match self {
            EdgeType::RelatesTo => "relates_to",
            EdgeType::IsA => "is_a",
            EdgeType::PartOf => "part_of",
            EdgeType::LocatedIn => "located_in",
            EdgeType::HappenedAt => "happened_at",
            EdgeType::CausedBy => "caused_by",
            EdgeType::SimilarTo => "similar_to",
            EdgeType::References => "references",
            EdgeType::Prefers => "prefers",
            EdgeType::Custom(s) => s,
        }
    }
}

impl From<String> for EdgeType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "relates_to" => EdgeType::RelatesTo,
            "is_a" => EdgeType::IsA,
            "part_of" => EdgeType::PartOf,
            "located_in" => EdgeType::LocatedIn,
            "happened_at" => EdgeType::HappenedAt,
            "caused_by" => EdgeType::CausedBy,
            "similar_to" => EdgeType::SimilarTo,
            "references" => EdgeType::References,
            "prefers" => EdgeType::Prefers,
            _ => EdgeType::Custom(s),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphBackup {
    pub version: u32,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub created_at: u64,
}
