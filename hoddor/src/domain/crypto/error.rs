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

impl CryptoError {
    pub fn key_derivation_error(message: impl Into<String>) -> Self {
        CryptoError::KeyDerivationError(message.into())
    }

    pub fn encryption_error(message: impl Into<String>) -> Self {
        CryptoError::EncryptionError(message.into())
    }

    pub fn decryption_error(message: impl Into<String>) -> Self {
        CryptoError::DecryptionError(message.into())
    }

    pub fn invalid_prf_output(message: impl Into<String>) -> Self {
        CryptoError::InvalidPrfOutput(message.into())
    }

    pub fn invalid_identity(message: impl Into<String>) -> Self {
        CryptoError::InvalidIdentity(message.into())
    }

    pub fn invalid_recipient(message: impl Into<String>) -> Self {
        CryptoError::InvalidRecipient(message.into())
    }
}
