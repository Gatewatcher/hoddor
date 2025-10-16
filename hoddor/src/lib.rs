extern crate console_error_panic_hook;

// Hexagonal architecture modules
pub mod domain;
pub mod ports;
pub mod adapters;
pub mod platform;

// Existing modules
pub mod crypto;
pub mod global;
pub mod measure;
pub mod notifications;
pub mod signaling;
pub mod sync;
pub mod vault;
pub mod webauthn;
pub mod webrtc;

// Re-exports for testing
pub use domain::vault::{IdentitySalts, NamespaceData, Vault, VaultMetadata};
pub use platform::Platform;
pub use vault::{read_vault_with_name, save_vault};

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start_app() -> Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    Ok(())
}
