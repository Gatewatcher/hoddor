/// Logger port - provides logging capabilities across platforms.
///
/// Abstracts logging from platform-specific implementations:
/// - WASM: Console API (console.log, console.error, etc.)
/// - Native: tracing crate or standard output
pub trait LoggerPort: Send + Sync {
    fn log(&self, message: &str);

    fn error(&self, message: &str);

    fn warn(&self, message: &str);

    fn time(&self, label: &str);

    fn time_end(&self, label: &str);
}
