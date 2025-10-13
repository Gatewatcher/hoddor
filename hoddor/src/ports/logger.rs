/// Logger port - provides logging capabilities across platforms.
///
/// Abstracts logging from platform-specific implementations:
/// - WASM: Console API (console.log, console.error, etc.)
/// - Native: tracing crate or standard output
pub trait LoggerPort: Send + Sync {
    /// Log an informational message.
    fn log(&self, message: &str);

    /// Log an error message.
    fn error(&self, message: &str);

    /// Log a warning message.
    fn warn(&self, message: &str);

    /// Start a performance timer with the given label.
    fn time(&self, label: &str);

    /// End a performance timer and log the elapsed duration.
    fn time_end(&self, label: &str);
}
