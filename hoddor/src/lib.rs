extern crate console_error_panic_hook;

pub mod adapters;
pub mod domain;
pub mod facades;
pub mod platform;
pub mod ports;

pub mod notifications;

#[cfg(target_arch = "wasm32")]
pub mod global;
#[cfg(target_arch = "wasm32")]
pub mod measure;
#[cfg(target_arch = "wasm32")]
pub mod signaling;
#[cfg(target_arch = "wasm32")]
pub mod sync;
#[cfg(target_arch = "wasm32")]
pub mod webrtc;

#[cfg(target_arch = "wasm32")]
pub use facades::wasm::{crypto, webauthn};

#[cfg(all(target_arch = "wasm32", feature = "graph"))]
pub use facades::wasm::graph;

pub use domain::vault::{IdentitySalts, NamespaceData, Vault, VaultMetadata};
pub use platform::Platform;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start_app() -> Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    Ok(())
}
