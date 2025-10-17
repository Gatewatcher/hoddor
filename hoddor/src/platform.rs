/// Platform - Dependency injection container for all ports.
///
/// Stores concrete adapter instances directly.
/// Platform selection happens at compile-time via #[cfg].
use crate::adapters::{
    AgeEncryption, AgeIdentity, Argon2Kdf, Clock, ConsoleLogger, Locks, Notifier, Persistence, Prf,
    Storage,
};
use crate::ports::{
    ClockPort, EncryptionPort, IdentityPort, KeyDerivationPort, LockPort, LoggerPort, NotifierPort,
    PersistencePort, PrfPort, StoragePort,
};

#[derive(Clone, Copy)]
pub struct Platform {
    clock: Clock,
    logger: ConsoleLogger,
    locks: Locks,
    notifier: Notifier,
    persistence: Persistence,
    storage: Storage,
    encryption: AgeEncryption,
    identity: AgeIdentity,
    kdf: Argon2Kdf,
    prf: Prf,
}

impl Platform {
    pub fn new() -> Self {
        Self {
            clock: Clock::new(),
            logger: ConsoleLogger::new(),
            locks: Locks::new(),
            notifier: Notifier::new(),
            persistence: Persistence::new(),
            storage: Storage::new(),
            encryption: AgeEncryption::new(),
            identity: AgeIdentity::new(),
            kdf: Argon2Kdf::new(),
            prf: Prf::new(),
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

    #[inline]
    pub fn storage(&self) -> &dyn StoragePort {
        &self.storage
    }

    #[inline]
    pub fn notifier(&self) -> &dyn NotifierPort {
        &self.notifier
    }

    #[inline]
    pub fn encryption(&self) -> &dyn EncryptionPort {
        &self.encryption
    }

    #[inline]
    pub fn identity(&self) -> &dyn IdentityPort {
        &self.identity
    }

    #[inline]
    pub fn kdf(&self) -> &dyn KeyDerivationPort {
        &self.kdf
    }

    #[inline]
    pub fn prf(&self) -> &dyn PrfPort {
        &self.prf
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
        logger.log("test");
    }

    #[test]
    fn test_platform_clock_access() {
        let platform = Platform::new();
        let clock = platform.clock();
        assert!(clock.is_available(), "Clock should be accessible");
        let _timestamp = clock.now();
    }

    #[test]
    fn test_platform_persistence_access() {
        let platform = Platform::new();
        let persistence = platform.persistence();
        let _has_requested = persistence.has_requested();
    }

    #[test]
    fn test_platform_locks_access() {
        let platform = Platform::new();
        let _locks = platform.locks();
    }

    #[test]
    fn test_platform_storage_access() {
        let platform = Platform::new();
        let _storage = platform.storage();
    }

    #[test]
    fn test_platform_crypto_access() {
        let platform = Platform::new();
        let _encryption = platform.encryption();
        let _identity = platform.identity();
        let _kdf = platform.kdf();
        let _prf = platform.prf();
    }

    #[test]
    fn test_platform_prf_availability() {
        let platform = Platform::new();
        let prf = platform.prf();
        let _ = prf.is_available();
    }
}
