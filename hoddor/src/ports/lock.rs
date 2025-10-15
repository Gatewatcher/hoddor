use crate::errors::VaultError;
use async_trait::async_trait;

/// Lock guard. Released automatically when dropped (RAII).
pub trait LockGuard {}

/// Port for exclusive locks.
#[async_trait(?Send)]
pub trait LockPort: Send + Sync {
    /// Acquires a lock. Retries with exponential backoff if unavailable.
    async fn acquire(&self, name: &str) -> Result<Box<dyn LockGuard>, VaultError>;
}
