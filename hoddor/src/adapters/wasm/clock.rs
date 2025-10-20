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

    #[wasm_bindgen_test]
    fn test_is_available_consistency() {
        let clock = Clock::new();

        let available1 = clock.is_available();
        let available2 = clock.is_available();

        assert_eq!(available1, available2, "is_available should be consistent");
        assert!(available1, "Clock should be available in browser/worker");
    }

    #[wasm_bindgen_test]
    fn test_multiple_clock_instances() {
        let clock1 = Clock::new();
        let clock2 = Clock::new();

        assert!(clock1.is_available(), "First clock should be available");
        assert!(clock2.is_available(), "Second clock should be available");

        let t1 = clock1.now();
        let t2 = clock2.now();

        assert!(t1 > 0.0, "First clock should return positive time");
        assert!(t2 >= t1, "Second clock reading should be >= first");
    }

    #[wasm_bindgen_test]
    fn test_clock_high_precision() {
        let clock = Clock::new();
        let timestamp = clock.now();

        assert!(timestamp > 0.0, "Timestamp should be positive");
        assert!(
            timestamp < 1e15,
            "Timestamp should be reasonable (not an absurd value)"
        );
    }

    #[wasm_bindgen_test]
    fn test_clock_is_copy() {
        let clock1 = Clock::new();
        let clock2 = clock1;

        assert!(clock1.is_available());
        assert!(clock2.is_available());

        let _t1 = clock1.now();
        let _t2 = clock2.now();
    }
}
