use crate::ports::clock::ClockPort;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
pub struct Clock;

impl Clock {
    pub fn new() -> Self {
        Self
    }
}

impl ClockPort for Clock {
    fn now(&self) -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64
    }

    fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_creation() {
        let clock = Clock::new();
        assert!(
            clock.is_available(),
            "Native clock should always be available"
        );
    }

    #[test]
    fn test_clock_now_returns_valid_unix_timestamp() {
        let clock = Clock::new();
        let timestamp = clock.now();

        assert!(
            timestamp > 1_577_836_800_000.0,
            "Timestamp should be after 2020: {}",
            timestamp
        );

        assert!(
            timestamp < 4_102_444_800_000.0,
            "Timestamp should be before 2100: {}",
            timestamp
        );
    }

    #[test]
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

    #[test]
    fn test_clock_always_available() {
        let clock = Clock::new();
        assert!(clock.is_available());
    }

    #[test]
    fn test_system_time_precision() {
        let clock = Clock::new();
        let t1 = clock.now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = clock.now();

        let elapsed = t2 - t1;
        assert!(
            elapsed >= 10.0,
            "Should have elapsed at least 10ms, got: {}ms",
            elapsed
        );
        assert!(elapsed < 50.0, "Elapsed time seems too high: {}ms", elapsed);
    }
}
