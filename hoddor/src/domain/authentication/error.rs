use std::fmt;

#[derive(Debug, Clone)]
pub enum AuthenticationError {
    /// Error during identity derivation from passphrase
    DerivationFailed(String),
    /// Invalid identity format
    InvalidIdentityFormat(String),
    /// Invalid passphrase
    InvalidPassphrase(String),
    /// Invalid salt
    InvalidSalt(String),
    /// Random generation error
    RandomGenerationFailed(String),
    /// No identity found
    IdentityNotFound,
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DerivationFailed(msg) => write!(f, "Identity derivation failed: {}", msg),
            Self::InvalidIdentityFormat(msg) => write!(f, "Invalid identity format: {}", msg),
            Self::InvalidPassphrase(msg) => write!(f, "Invalid passphrase: {}", msg),
            Self::InvalidSalt(msg) => write!(f, "Invalid salt: {}", msg),
            Self::RandomGenerationFailed(msg) => write!(f, "Random generation failed: {}", msg),
            Self::IdentityNotFound => write!(f, "Identity not found"),
        }
    }
}

impl std::error::Error for AuthenticationError {}
