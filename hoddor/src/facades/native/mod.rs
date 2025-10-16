pub mod crypto;
pub mod vault;

pub use vault::VaultManager;
pub use crypto::{generate_identity, IdentityHandle, RecipientHandle, CryptoError};
