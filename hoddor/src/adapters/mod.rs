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

#[cfg(feature = "graph")]
#[path = "wasm/cozo_graph.rs"]
mod cozo_graph;

#[cfg(feature = "graph")]
pub use cozo_graph::CozoGraphAdapter as Graph;
