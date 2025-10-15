/// Adapters module - platform-specific implementations of ports.

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub use wasm::{Clock, ConsoleLogger, Locks, Persistence};
#[cfg(not(target_arch = "wasm32"))]
pub use native::{Clock, ConsoleLogger, Locks, Persistence};
