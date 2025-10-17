/// Crypto ports - Defines the interfaces for cryptographic operations.
///
/// These traits abstract cryptographic functionality from specific implementations:
/// - EncryptionPort: Age encryption/decryption
/// - KeyDerivationPort: Argon2 key derivation
/// - IdentityPort: Age identity management
/// - PrfPort: WebAuthn PRF (WASM only, stub in native)
use async_trait::async_trait;
use std::error::Error;

/// Port for encryption/decryption operations
#[async_trait(?Send)]
pub trait EncryptionPort: Send + Sync {
    /// Encrypt data for multiple recipients
    async fn encrypt(&self, data: &[u8], recipients: &[&str]) -> Result<Vec<u8>, Box<dyn Error>>;

    /// Decrypt data with an identity (private key string)
    async fn decrypt(&self, encrypted: &[u8], identity: &str) -> Result<Vec<u8>, Box<dyn Error>>;
}

/// Port for key derivation operations
#[async_trait(?Send)]
pub trait KeyDerivationPort: Send + Sync {
    /// Derive a 32-byte seed from a passphrase using Argon2
    async fn derive_from_passphrase(
        &self,
        passphrase: &str,
        salt: &[u8],
    ) -> Result<[u8; 32], Box<dyn Error>>;
}

/// Port for identity management
pub trait IdentityPort: Send + Sync {
    /// Generate a new random identity
    fn generate(&self) -> Result<String, Box<dyn Error>>;

    /// Create identity from a 32-byte seed
    fn from_seed(&self, seed: [u8; 32]) -> Result<String, Box<dyn Error>>;

    /// Parse a recipient public key
    fn parse_recipient(&self, recipient: &str) -> Result<String, Box<dyn Error>>;

    /// Get public key from private identity
    fn to_public(&self, identity: &str) -> Result<String, Box<dyn Error>>;
}

/// Port for PRF (Pseudo-Random Function) operations
/// Only available in WASM (WebAuthn), stub in native
pub trait PrfPort: Send + Sync {
    /// Derive a 32-byte key from PRF outputs
    fn derive_from_prf(&self, first: &[u8], second: &[u8]) -> Result<[u8; 32], Box<dyn Error>>;

    /// Check if PRF is available on this platform
    fn is_available(&self) -> bool;
}
