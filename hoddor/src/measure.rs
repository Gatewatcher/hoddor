use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::prelude::*;

pub static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

#[wasm_bindgen]
pub fn set_debug_mode(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::SeqCst);
}

#[macro_export]
macro_rules! time_it {
    ($label:expr, $block:expr) => {{
        let debug = $crate::measure::DEBUG_MODE.load(std::sync::atomic::Ordering::SeqCst);
        if debug {
            let platform = $crate::platform::Platform::new();
            if platform.clock().is_available() {
                platform.logger().time($label);
            }
        }
        let result = $block;
        if debug {
            let platform = $crate::platform::Platform::new();
            if platform.clock().is_available() {
                platform.logger().time_end($label);
            }
        }
        result
    }};
}

pub use crate::time_it;
