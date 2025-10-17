use crate::global::get_global_scope;
use crate::ports::clock::ClockPort;
use wasm_bindgen::JsCast;
use web_sys::{Performance, WorkerGlobalScope};

#[derive(Clone, Copy)]
pub struct Clock;

impl Clock {
    pub fn new() -> Self {
        Self
    }

    fn get_performance(&self) -> Option<Performance> {
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
}

impl ClockPort for Clock {
    fn now(&self) -> f64 {
        if let Some(perf) = self.get_performance() {
            perf.now()
        } else {
            0.0
        }
    }

    fn is_available(&self) -> bool {
        self.get_performance().is_some()
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_clock_creation() {
        let clock = Clock::new();
        assert!(clock.is_available(), "Clock should be available");
    }

    #[wasm_bindgen_test]
    fn test_clock_now_returns_positive() {
        let clock = Clock::new();
        let timestamp = clock.now();
        assert!(
            timestamp > 0.0,
            "Timestamp should be positive: {}",
            timestamp
        );
    }

    #[wasm_bindgen_test]
    fn test_clock_monotonic_time() {
        let clock = Clock::new();
        let t1 = clock.now();

        let mut sum = 0;
        for i in 0..1000 {
            sum += i;
        }
        let _ = sum;

        let t2 = clock.now();
        assert!(t2 >= t1, "Time should be monotonic (t1={}, t2={})", t1, t2);
    }

    #[wasm_bindgen_test]
    fn test_performance_api_available() {
        let clock = Clock::new();
        assert!(
            clock.get_performance().is_some(),
            "Performance API should be available in browser/worker"
        );
    }
}
