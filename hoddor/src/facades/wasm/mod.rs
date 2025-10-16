pub mod converters;
pub mod crypto;
pub mod legacy;
pub mod vault;
pub mod webauthn;

// Re-export legacy functions for backward compatibility
pub use legacy::{configure_cleanup, read_vault_with_name, save_vault, update_vault_from_sync};
