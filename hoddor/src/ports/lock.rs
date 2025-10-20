use crate::domain::vault::error::VaultError;
use async_trait::async_trait;

pub trait LockGuard {}

#[async_trait(?Send)]
pub trait LockPort: Send + Sync {
    async fn acquire(&self, name: &str) -> Result<Box<dyn LockGuard>, VaultError>;
}
