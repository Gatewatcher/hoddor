use crate::domain::vault::error::VaultError;
use async_trait::async_trait;

/// Port for storage persistence operations.
#[async_trait(?Send)]
pub trait PersistencePort: Send + Sync {
    fn has_requested(&self) -> bool;

    async fn request(&self) -> Result<bool, VaultError>;

    async fn check(&self) -> Result<bool, VaultError>;
}
