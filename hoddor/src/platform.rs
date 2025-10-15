/// Platform - Dependency injection container for all ports.
///
/// Stores concrete adapter instances directly.
/// Platform selection happens at compile-time via #[cfg].

use crate::adapters::{Clock, ConsoleLogger, Locks, Persistence};
use crate::ports::{ClockPort, LockPort, LoggerPort, PersistencePort};

#[derive(Clone, Copy)]
pub struct Platform {
    clock: Clock,
    logger: ConsoleLogger,
    locks: Locks,
    persistence: Persistence,
}

impl Platform {
    /// Creates a new Platform with default adapters for the current target.
    pub fn new() -> Self {
        Self {
            clock: Clock::new(),
            logger: ConsoleLogger::new(),
            locks: Locks::new(),
            persistence: Persistence::new(),
        }
    }

    #[inline]
    pub fn clock(&self) -> &dyn ClockPort {
        &self.clock
    }

    #[inline]
    pub fn logger(&self) -> &dyn LoggerPort {
        &self.logger
    }

    #[inline]
    pub fn locks(&self) -> &dyn LockPort {
        &self.locks
    }

    #[inline]
    pub fn persistence(&self) -> &dyn PersistencePort {
        &self.persistence
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_creation() {
        let platform = Platform::new();
        platform.logger().log("test");
    }

    #[test]
    fn test_platform_clone() {
        let platform = Platform::new();
        let cloned = platform.clone();
        cloned.logger().log("test clone");
    }

    #[test]
    fn test_platform_default() {
        let platform = Platform::default();
        platform.logger().log("test default");
    }

    #[test]
    fn test_platform_logger_access() {
        let platform = Platform::new();
        let logger = platform.logger();
        logger.log("test"); // Verify we can call without panic
    }

    #[test]
    fn test_platform_clock_access() {
        let platform = Platform::new();
        let clock = platform.clock();
        assert!(clock.is_available(), "Clock should be accessible");
        let _timestamp = clock.now(); // Verify we can call now() without panic
    }

    #[test]
    fn test_platform_persistence_access() {
        let platform = Platform::new();
        let persistence = platform.persistence();
        let _has_requested = persistence.has_requested(); // Verify we can call without panic
    }

    #[test]
    fn test_platform_locks_access() {
        let platform = Platform::new();
        let _locks = platform.locks();
    }
}
