/// Adapters module - platform-specific implementations of ports.

pub mod shared;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

// Re-export shared adapters (work on both platforms)
pub use shared::{AgeEncryption, AgeIdentity, Argon2Kdf};

#[cfg(target_arch = "wasm32")]
pub use wasm::{Clock, ConsoleLogger, Locks, Notifier, OPFSStorage as Storage, Persistence, WebAuthnPrf as Prf};
#[cfg(not(target_arch = "wasm32"))]
pub use native::{Clock, ConsoleLogger, FsStorage as Storage, Locks, MockPrf as Prf, Notifier, Persistence};
