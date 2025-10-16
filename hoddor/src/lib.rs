extern crate console_error_panic_hook;

// Hexagonal architecture modules
pub mod domain;
pub mod ports;
pub mod adapters;
pub mod platform;
pub mod facades;

// Existing modules
pub mod notifications;

// WASM-only modules
#[cfg(target_arch = "wasm32")]
pub mod global;
#[cfg(target_arch = "wasm32")]
pub mod measure;
#[cfg(target_arch = "wasm32")]
pub mod signaling;
#[cfg(target_arch = "wasm32")]
pub mod sync;
#[cfg(target_arch = "wasm32")]
pub mod vault;
#[cfg(target_arch = "wasm32")]
pub mod webrtc;

// Re-export crypto and webauthn from facades
#[cfg(target_arch = "wasm32")]
pub use facades::wasm::{crypto, webauthn};

// Re-exports for testing
pub use domain::vault::{IdentitySalts, NamespaceData, Vault, VaultMetadata};
pub use platform::Platform;

// WASM-only re-exports
#[cfg(target_arch = "wasm32")]
pub use vault::{read_vault_with_name, save_vault};

// WASM initialization
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start_app() -> Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    Ok(())
}
