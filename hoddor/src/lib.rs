extern crate console_error_panic_hook;

pub mod console;
pub mod crypto;
pub mod errors;
pub mod file_system;
pub mod global;
pub mod lock;
pub mod measure;
pub mod persistence;
pub mod signaling;
pub mod sync;
pub mod vault;
pub mod webauthn;
pub mod webrtc;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start_app() -> Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    Ok(())
}
