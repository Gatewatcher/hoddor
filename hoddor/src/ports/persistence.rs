use crate::errors::VaultError;
use async_trait::async_trait;

/// Port for storage persistence operations.
#[async_trait(?Send)]
pub trait PersistencePort: Send + Sync {
    /// Check if persistence has been requested during this session.
    fn has_requested(&self) -> bool;

    /// Request persistent storage from the browser.
    async fn request(&self) -> Result<bool, VaultError>;

    /// Check if storage is currently persisted.
    async fn check(&self) -> Result<bool, VaultError>;
}
