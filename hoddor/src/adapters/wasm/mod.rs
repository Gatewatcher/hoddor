/// WASM adapters - implementations using browser APIs.

pub mod clock;
pub mod console_logger;
pub mod persistence;

pub use clock::Clock;
pub use console_logger::ConsoleLogger;
pub use persistence::Persistence;
