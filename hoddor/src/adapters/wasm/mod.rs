/// WASM adapters - implementations using browser APIs.

pub mod clock;
pub mod console_logger;

pub use clock::Clock;
pub use console_logger::ConsoleLogger;
