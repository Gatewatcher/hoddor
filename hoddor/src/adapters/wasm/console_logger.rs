use crate::ports::LoggerPort;
use wasm_bindgen::prelude::*;

// FFI bindings to JavaScript console API
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

/// WASM logger implementation using the browser Console API.
///
/// Forwards all logging calls to the JavaScript console object.
/// This is a zero-cost abstraction over the console FFI.
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
