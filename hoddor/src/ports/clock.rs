pub trait ClockPort: Send + Sync {
    fn now(&self) -> f64;

    fn is_available(&self) -> bool;
}
