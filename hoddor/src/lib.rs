extern crate console_error_panic_hook;

pub mod console;
pub mod crypto;
mod errors;
mod file_system;
mod global;
mod lock;
mod measure;
pub mod vault;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    // Set a panic hook for clearer errors in the console.
    console_error_panic_hook::set_once();
    console::log("Worker started (File System Access API assumed available).");
}
