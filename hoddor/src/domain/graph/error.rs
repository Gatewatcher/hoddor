#[derive(Debug)]
pub enum GraphError {
    NodeNotFound(String),
    EdgeNotFound(String),
    NodeAlreadyExists(String),
    InvalidNodeType(String),
    InvalidEdgeType(String),
    EncryptionError(String),
    DecryptionError(String),
    SerializationError(String),
    DatabaseError(String),
    IntegrityError(String),
    InvalidEmbedding(String),
    VaultMismatch { expected: String, found: String },
    Other(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            GraphError::EdgeNotFound(id) => write!(f, "Edge not found: {}", id),
            GraphError::NodeAlreadyExists(id) => write!(f, "Node already exists: {}", id),
            GraphError::InvalidNodeType(t) => write!(f, "Invalid node type: {}", t),
            GraphError::InvalidEdgeType(t) => write!(f, "Invalid edge type: {}", t),
            GraphError::EncryptionError(e) => write!(f, "Encryption error: {}", e),
            GraphError::DecryptionError(e) => write!(f, "Decryption error: {}", e),
            GraphError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            GraphError::DatabaseError(e) => write!(f, "Database error: {}", e),
            GraphError::IntegrityError(e) => write!(f, "Integrity verification failed: {}", e),
            GraphError::InvalidEmbedding(e) => write!(f, "Invalid embedding: {}", e),
            GraphError::VaultMismatch { expected, found } => {
                write!(
                    f,
                    "Vault mismatch: expected '{}', found '{}'",
                    expected, found
                )
            }
            GraphError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for GraphError {}

pub type GraphResult<T> = Result<T, GraphError>;
