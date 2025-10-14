/// Platform - Dependency injection container for all ports.
///
/// Hybrid approach:
/// - Stateless ports: `&'static` references (zero-cost)
/// - Stateful ports: `Arc<dyn Trait>` (ref-counted, when needed)

use crate::ports::LoggerPort;

#[derive(Clone, Copy)]
pub struct Platform {
    logger: &'static dyn LoggerPort,
}

impl Platform {
    /// Creates a new Platform with default adapters for the current target.
    pub fn new() -> Self {
        Self {
            logger: crate::adapters::logger(),
        }
    }

    #[inline]
    pub fn logger(&self) -> &'static dyn LoggerPort {
        self.logger
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
        logger.log("test 1");
        logger.warn("test 2");
        logger.error("test 3");
    }
}
