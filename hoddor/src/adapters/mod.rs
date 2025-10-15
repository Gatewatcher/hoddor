/// Adapters module - platform-specific implementations of ports.

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub use wasm::{Clock, ConsoleLogger, Locks, OPFSStorage as Storage, Persistence};
#[cfg(not(target_arch = "wasm32"))]
pub use native::{Clock, ConsoleLogger, FsStorage as Storage, Locks, Persistence};
