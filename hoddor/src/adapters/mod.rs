/// Adapters module - platform-specific implementations of ports.

pub mod global_clock;
pub mod global_logger;
pub mod global_persistence;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub use wasm::{Clock, ConsoleLogger, Persistence};
#[cfg(not(target_arch = "wasm32"))]
pub use native::{Clock, ConsoleLogger, Persistence};

pub use global_clock::clock;
pub use global_logger::logger;
pub use global_persistence::persistence;
