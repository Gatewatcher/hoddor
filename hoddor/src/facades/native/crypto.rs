use crate::domain::crypto;
use crate::platform::Platform;
/// Native facade for cryptographic operations
/// Provides pure Rust API that delegates to domain logic
use age::{
    secrecy::ExposeSecret,
    x25519::{Identity, Recipient},
};
use std::fmt;

#[derive(Debug, Clone)]
pub enum CryptoError {
    GenerationFailed(String),
    ParseFailed(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::GenerationFailed(msg) => write!(f, "Identity generation failed: {}", msg),
            CryptoError::ParseFailed(msg) => write!(f, "Parse failed: {}", msg),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Generate a new Age identity (key pair)
/// Returns (public_key, private_key) as strings
pub fn generate_identity() -> Result<(String, String), CryptoError> {
    let platform = Platform::new();

    let identity_str = crypto::generate_identity(&platform)
        .map_err(|e| CryptoError::GenerationFailed(e.to_string()))?;

    let identity: Identity = identity_str
        .parse()
        .map_err(|e| CryptoError::ParseFailed(format!("Failed to parse identity: {}", e)))?;

    let public_key = identity.to_public().to_string();
    let private_key = identity.to_string().expose_secret().to_string();

    Ok((public_key, private_key))
}

/// Handle for an Age recipient (public key)
#[derive(Clone)]
pub struct RecipientHandle {
    recipient: Recipient,
}

impl fmt::Debug for RecipientHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecipientHandle")
            .field("public_key", &self.recipient.to_string())
            .finish()
    }
}

impl fmt::Display for RecipientHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.recipient)
    }
}

impl RecipientHandle {
    /// Get the recipient as a string
    pub fn to_string(&self) -> String {
        self.recipient.to_string()
    }

    /// Parse a recipient from a string
    pub fn from_string(s: &str) -> Result<Self, CryptoError> {
        let recipient: Recipient = s
            .parse()
            .map_err(|e| CryptoError::ParseFailed(format!("Failed to parse recipient: {}", e)))?;
        Ok(Self { recipient })
    }
}

impl From<Recipient> for RecipientHandle {
    fn from(recipient: Recipient) -> Self {
        Self { recipient }
    }
}

impl AsRef<Recipient> for RecipientHandle {
    fn as_ref(&self) -> &Recipient {
        &self.recipient
    }
}

/// Handle for an Age identity (private key)
#[derive(Clone)]
pub struct IdentityHandle {
    identity: Identity,
}

impl fmt::Debug for IdentityHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IdentityHandle {{ public_key: {} }}",
            self.identity.to_public()
        )
    }
}

impl fmt::Display for IdentityHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identity.to_public())
    }
}

impl AsRef<dyn age::Identity + 'static> for IdentityHandle {
    fn as_ref(&self) -> &(dyn age::Identity + 'static) {
        &self.identity
    }
}

impl IdentityHandle {
    /// Get the public key as a string
    pub fn public_key(&self) -> String {
        self.identity.to_public().to_string()
    }

    /// Get the recipient handle (public key wrapper)
    pub fn to_public(&self) -> RecipientHandle {
        RecipientHandle::from(self.identity.to_public())
    }

    /// Get the private key as a string
    pub fn private_key(&self) -> String {
        self.identity.to_string().expose_secret().to_string()
    }

    /// Create identity handle from private key string
    pub fn from_private_key(private_key: &str) -> Result<Self, CryptoError> {
        let identity = private_key
            .parse::<Identity>()
            .map_err(|e| CryptoError::ParseFailed(format!("Failed to parse identity: {}", e)))?;

        Ok(IdentityHandle::from(identity))
    }

    /// Get both keys as a tuple (public_key, private_key)
    pub fn keys(&self) -> (String, String) {
        (self.public_key(), self.private_key())
    }
}

impl From<Identity> for IdentityHandle {
    fn from(identity: Identity) -> Self {
        IdentityHandle { identity }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let result = generate_identity();
        assert!(result.is_ok());

        let (public_key, private_key) = result.unwrap();
        assert!(public_key.starts_with("age1"));
        assert!(private_key.starts_with("AGE-SECRET-KEY-"));
    }

    #[test]
    fn test_identity_handle_keys() {
        let (public_key, private_key) = generate_identity().unwrap();
        let identity = IdentityHandle::from_private_key(&private_key).unwrap();

        assert_eq!(identity.public_key(), public_key);
        assert_eq!(identity.private_key(), private_key);
    }

    #[test]
    fn test_recipient_handle() {
        let (public_key, _) = generate_identity().unwrap();
        let recipient = RecipientHandle::from_string(&public_key).unwrap();

        assert_eq!(recipient.to_string(), public_key);
    }
}
