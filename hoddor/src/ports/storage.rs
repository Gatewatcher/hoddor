use crate::domain::vault::error::VaultError;
use async_trait::async_trait;

/// Port for file system storage operations.
#[async_trait(?Send)]
pub trait StoragePort: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<String, VaultError>;

    async fn write_file(&self, path: &str, content: &str) -> Result<(), VaultError>;

    async fn delete_file(&self, path: &str) -> Result<(), VaultError>;

    async fn create_directory(&self, path: &str) -> Result<(), VaultError>;

    async fn delete_directory(&self, path: &str) -> Result<(), VaultError>;

    async fn directory_exists(&self, path: &str) -> Result<bool, VaultError>;

    async fn list_entries(&self, path: &str) -> Result<Vec<String>, VaultError>;
}
