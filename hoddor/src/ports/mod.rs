/// Ports module - Defines the interfaces (traits) that abstract platform-specific functionality.
///
/// This module contains all the port traits that define contracts between the domain layer
/// and the infrastructure adapters. These traits enable the hexagonal architecture by
/// decoupling the business logic from platform-specific implementations.

pub mod clock;
pub mod crypto;
pub mod lock;
pub mod logger;
pub mod notifier;
pub mod persistence;
pub mod storage;

pub use clock::ClockPort;
pub use crypto::{EncryptionPort, IdentityPort, KeyDerivationPort, PrfPort};
pub use lock::{LockGuard, LockPort};
pub use logger::LoggerPort;
pub use notifier::NotifierPort;
pub use persistence::PersistencePort;
pub use storage::StoragePort;
