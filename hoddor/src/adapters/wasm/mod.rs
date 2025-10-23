mod error_conversions;

pub mod clock;
pub mod console_logger;
pub mod locks;
pub mod notifier;
pub mod opfs_storage;
pub mod persistence;
pub mod webauthn_prf;

#[cfg(feature = "graph")]
pub mod cozo_graph;
#[cfg(feature = "graph")]
pub mod graph_persistence;
#[cfg(feature = "graph")]
pub mod simple_graph;

pub use clock::Clock;
pub use console_logger::ConsoleLogger;
pub use locks::Locks;
pub use notifier::Notifier;
pub use opfs_storage::OpfsStorage;
pub use persistence::Persistence;
pub use webauthn_prf::WebAuthnPrf;

#[cfg(feature = "graph")]
pub use cozo_graph::CozoGraphAdapter;
#[cfg(feature = "graph")]
pub use graph_persistence::{EncryptionConfig, GraphPersistence};
#[cfg(feature = "graph")]
pub use simple_graph::SimpleGraphAdapter;
