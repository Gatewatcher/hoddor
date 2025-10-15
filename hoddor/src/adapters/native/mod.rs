/// Native adapters - implementations for native Rust (non-WASM).

pub mod clock;
pub mod console_logger;
pub mod fs_storage;
pub mod locks;
pub mod persistence;

pub use clock::Clock;
pub use console_logger::ConsoleLogger;
pub use fs_storage::FsStorage;
pub use locks::Locks;
pub use persistence::Persistence;
