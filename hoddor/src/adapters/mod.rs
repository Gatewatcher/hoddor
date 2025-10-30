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

#[cfg(all(feature = "graph-simple", not(feature = "graph-cozo")))]
#[path = "wasm/simple_graph.rs"]
mod simple_graph;

#[cfg(all(feature = "graph-simple", not(feature = "graph-cozo")))]
pub use simple_graph::SimpleGraphAdapter as Graph;

#[cfg(all(feature = "graph-cozo", not(feature = "graph-simple")))]
#[path = "wasm/cozo_graph.rs"]
mod cozo_graph;

#[cfg(all(feature = "graph-cozo", not(feature = "graph-simple")))]
pub use cozo_graph::CozoGraphAdapter as Graph;
