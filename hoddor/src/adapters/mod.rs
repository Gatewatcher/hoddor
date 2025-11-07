#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::{
    Clock, ConsoleLogger, Locks, Notifier, OpfsStorage as Storage, Persistence, WebAuthnPrf as Prf,
};

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{
    Clock, ConsoleLogger, FsStorage as Storage, Locks, MockPrf as Prf, Notifier, Persistence,
};

pub mod shared;
pub use shared::{AgeEncryption, AgeIdentity, Argon2Kdf};

#[cfg(all(feature = "graph", target_arch = "wasm32"))]
pub use wasm::CozoGraphAdapter as Graph;
