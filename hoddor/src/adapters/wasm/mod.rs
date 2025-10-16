/// WASM adapters - implementations using browser APIs.

mod error_conversions; // JsValue → VaultError conversion for WASM

pub mod clock;
pub mod console_logger;
pub mod locks;
pub mod notifier;
pub mod opfs_storage;
pub mod persistence;
pub mod webauthn_prf;

pub use clock::Clock;
pub use console_logger::ConsoleLogger;
pub use locks::Locks;
pub use notifier::Notifier;
pub use opfs_storage::OPFSStorage;
pub use persistence::Persistence;
pub use webauthn_prf::WebAuthnPrf;
