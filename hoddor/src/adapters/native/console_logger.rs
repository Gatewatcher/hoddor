use crate::ports::LoggerPort;

/// Native logger implementation using stdout/stderr.
///
/// This is a simple implementation that prints to the console.
/// In the future, this could be replaced with a more sophisticated
/// logging framework like `tracing` or `log`.
#[derive(Debug, Clone, Copy)]
pub struct ConsoleLogger;

impl ConsoleLogger {
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
        println!("[LOG] {message}");
    }

    fn error(&self, message: &str) {
        eprintln!("[ERROR] {message}");
    }

    fn warn(&self, message: &str) {
        eprintln!("[WARN] {message}");
    }

    fn time(&self, label: &str) {
        println!("[TIME:START] {label}");
    }

    fn time_end(&self, label: &str) {
        println!("[TIME:END] {label}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_creation() {
        let logger = ConsoleLogger::new();
        logger.log("test");
    }

    #[test]
    fn test_logger_all_methods() {
        let logger = ConsoleLogger::new();
        logger.log("test log");
        logger.warn("test warn");
        logger.error("test error");
        logger.time("test_timer");
        logger.time_end("test_timer");
    }
}
