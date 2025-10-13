/// Global logger instance - automatically selects implementation based on build target.
///
/// Supports both WASM (browser console) and native (stdout/stderr) platforms.
/// The correct implementation is selected at compile time based on the target architecture.

use crate::ports::LoggerPort;
use lazy_static::lazy_static;

#[cfg(target_arch = "wasm32")]
use crate::adapters::wasm::ConsoleLogger;
#[cfg(not(target_arch = "wasm32"))]
use crate::adapters::native::ConsoleLogger;

lazy_static! {
    pub static ref LOGGER: ConsoleLogger = ConsoleLogger::new();
}

/// Get the global logger instance.
#[inline]
pub fn logger() -> &'static dyn LoggerPort {
    &*LOGGER
}
