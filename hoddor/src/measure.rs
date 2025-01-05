use crate::global::get_global_scope;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::prelude::*;
use web_sys::{self, Performance, WorkerGlobalScope};

pub static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

#[wasm_bindgen]
pub fn set_debug_mode(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::SeqCst);
}

pub fn get_performance() -> Option<Performance> {
    match get_global_scope() {
        Ok(scope) => {
            if let Ok(worker) = scope.clone().dyn_into::<WorkerGlobalScope>() {
                worker.performance()
            } else if let Ok(window) = scope.dyn_into::<web_sys::Window>() {
                window.performance()
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

pub fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Some(perf) = window.performance() {
                return perf.now();
            }
        }
        0.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64
    }
}

#[macro_export]
macro_rules! time_it {
    ($label:expr, $block:expr) => {{
        let debug = $crate::measure::DEBUG_MODE.load(std::sync::atomic::Ordering::SeqCst);
        if debug {
            if let Some(_) = $crate::measure::get_performance() {
                $crate::console::time($label);
            }
        }
        let result = $block;
        if debug {
            if let Some(_) = $crate::measure::get_performance() {
                $crate::console::timeEnd($label);
            }
        }
        result
    }};
}

pub use crate::time_it;
