use async_trait::async_trait;
use std::error::Error;

#[async_trait(?Send)]
pub trait EncryptionPort: Send + Sync {
    async fn encrypt(&self, data: &[u8], recipients: &[&str]) -> Result<Vec<u8>, Box<dyn Error>>;

    async fn decrypt(&self, encrypted: &[u8], identity: &str) -> Result<Vec<u8>, Box<dyn Error>>;
}

#[async_trait(?Send)]
pub trait KeyDerivationPort: Send + Sync {
    async fn derive_from_passphrase(
        &self,
        passphrase: &str,
        salt: &[u8],
    ) -> Result<[u8; 32], Box<dyn Error>>;
}

pub trait IdentityPort: Send + Sync {
    fn generate(&self) -> Result<String, Box<dyn Error>>;

    fn from_seed(&self, seed: [u8; 32]) -> Result<String, Box<dyn Error>>;

    fn parse_recipient(&self, recipient: &str) -> Result<String, Box<dyn Error>>;

    fn to_public(&self, identity: &str) -> Result<String, Box<dyn Error>>;
}

pub trait PrfPort: Send + Sync {
    fn derive_from_prf(&self, first: &[u8], second: &[u8]) -> Result<[u8; 32], Box<dyn Error>>;

    fn is_available(&self) -> bool;
}
