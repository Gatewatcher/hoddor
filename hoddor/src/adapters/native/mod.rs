/// Native adapters - implementations for native Rust (non-WASM).

pub mod clock;
pub mod console_logger;
pub mod persistence;

pub use clock::Clock;
pub use console_logger::ConsoleLogger;
pub use persistence::Persistence;
