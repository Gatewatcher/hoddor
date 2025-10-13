/// Adapters module - platform-specific implementations of ports.

pub mod global_logger;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub use wasm::ConsoleLogger;
#[cfg(not(target_arch = "wasm32"))]
pub use native::ConsoleLogger;

pub use global_logger::logger;
