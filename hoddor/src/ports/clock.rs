/// Port for accessing time and performance measurement
pub trait ClockPort: Send + Sync {
    /// Returns the current timestamp in milliseconds
    fn now(&self) -> f64;

    /// Checks if performance timing is available
    fn is_available(&self) -> bool;
}
