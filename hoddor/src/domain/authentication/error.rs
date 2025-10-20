use std::fmt;

#[derive(Debug, Clone)]
pub enum AuthenticationError {
    DerivationFailed(String),
    InvalidIdentityFormat(String),
    InvalidPassphrase(String),
    InvalidSalt(String),
    RandomGenerationFailed(String),
    IdentityNotFound,
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DerivationFailed(msg) => write!(f, "Identity derivation failed: {msg}"),
            Self::InvalidIdentityFormat(msg) => write!(f, "Invalid identity format: {msg}"),
            Self::InvalidPassphrase(msg) => write!(f, "Invalid passphrase: {msg}"),
            Self::InvalidSalt(msg) => write!(f, "Invalid salt: {msg}"),
            Self::RandomGenerationFailed(msg) => write!(f, "Random generation failed: {msg}"),
            Self::IdentityNotFound => write!(f, "Identity not found"),
        }
    }
}

impl std::error::Error for AuthenticationError {}
