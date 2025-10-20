pub mod crypto;
pub mod vault;

pub use crypto::{generate_identity, CryptoError, IdentityHandle, RecipientHandle};
pub use vault::VaultManager;
