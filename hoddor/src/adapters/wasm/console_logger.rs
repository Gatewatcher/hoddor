use crate::ports::LoggerPort;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn time(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = timeEnd)]
    fn time_end(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
}

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
        log(message);
    }

    fn error(&self, message: &str) {
        error(message);
    }

    fn warn(&self, message: &str) {
        warn(message);
    }

    fn time(&self, label: &str) {
        time(label);
    }

    fn time_end(&self, label: &str) {
        time_end(label);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_logger_creation() {
        let logger = ConsoleLogger::new();
        logger.log("test");
    }

    #[wasm_bindgen_test]
    fn test_logger_all_methods() {
        let logger = ConsoleLogger::new();
        logger.log("test log");
        logger.warn("test warn");
        logger.error("test error");
        logger.time("test_timer");
        logger.time_end("test_timer");
    }
}
