use crate::ports::LoggerPort;

/// Native logger implementation using stdout/stderr.
///
/// This is a simple implementation that prints to the console.
/// In the future, this could be replaced with a more sophisticated
/// logging framework like `tracing` or `log`.
#[derive(Debug, Clone, Copy)]
pub struct ConsoleLogger;

impl ConsoleLogger {
    /// Create a new ConsoleLogger instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConsoleLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggerPort for ConsoleLogger {
    fn log(&self, message: &str) {
        println!("[LOG] {}", message);
    }

    fn error(&self, message: &str) {
        eprintln!("[ERROR] {}", message);
    }

    fn warn(&self, message: &str) {
        eprintln!("[WARN] {}", message);
    }

    fn time(&self, label: &str) {
        println!("[TIME:START] {}", label);
    }

    fn time_end(&self, label: &str) {
        println!("[TIME:END] {}", label);
    }
}
