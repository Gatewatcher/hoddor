use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a graph node
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

/// Unique identifier for a graph edge
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

/// A node in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier
    pub id: NodeId,

    /// Type of node (memory, entity, event, concept, etc.)
    pub node_type: String,

    /// Vault this node belongs to
    pub vault_id: String,

    /// Optional namespace
    pub namespace: Option<String>,

    /// Labels for categorization (non-encrypted)
    pub labels: Vec<String>,

    /// Embedding vector for similarity search (optional)
    pub embedding: Option<Vec<f32>>,

    /// Encrypted content (age encryption)
    pub encrypted_content: Vec<u8>,

    /// HMAC for integrity verification
    pub content_hmac: String,

    /// Metadata
    pub metadata: NodeMetadata,

    /// Timestamps
    pub created_at: u64,
    pub updated_at: u64,
    pub accessed_at: u64,

    /// Access counter
    pub access_count: u32,
}

/// Node metadata (non-sensitive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Size of content in bytes
    pub content_size: usize,

    /// Schema version for migrations
    pub version: u32,

    /// Optional expiration timestamp
    pub expires_at: Option<u64>,
}

/// A relationship between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Unique identifier
    pub id: EdgeId,

    /// Source node
    pub from_node: NodeId,

    /// Destination node
    pub to_node: NodeId,

    /// Type of relationship
    pub edge_type: String,

    /// Vault ID (must match nodes)
    pub vault_id: String,

    /// Edge properties
    pub properties: EdgeProperties,

    /// Creation timestamp
    pub created_at: u64,
}

/// Properties of an edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeProperties {
    /// Weight/strength of relationship (0.0 - 1.0)
    pub weight: f64,

    /// Is the relationship bidirectional?
    pub bidirectional: bool,

    /// Optional encrypted context
    pub encrypted_context: Option<Vec<u8>>,

    /// Additional metadata
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

/// Direction for edge traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeDirection {
    Incoming,
    Outgoing,
    Both,
}

/// Predefined node types
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

/// Predefined edge types
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
