use std::fmt;

#[derive(Debug, Clone)]
pub enum CryptoError {
    KeyDerivationError(String),
    EncryptionError(String),
    DecryptionError(String),
    InvalidPrfOutput(String),
    InvalidIdentity(String),
    InvalidRecipient(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::KeyDerivationError(msg) => write!(f, "Key derivation failed: {}", msg),
            CryptoError::EncryptionError(msg) => write!(f, "Encryption failed: {}", msg),
            CryptoError::DecryptionError(msg) => write!(f, "Decryption failed: {}", msg),
            CryptoError::InvalidPrfOutput(msg) => write!(f, "Invalid PRF output: {}", msg),
            CryptoError::InvalidIdentity(msg) => write!(f, "Invalid identity: {}", msg),
            CryptoError::InvalidRecipient(msg) => write!(f, "Invalid recipient: {}", msg),
        }
    }
}

impl std::error::Error for CryptoError {}
