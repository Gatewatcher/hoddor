use crate::errors::VaultError;
use async_trait::async_trait;

/// Port for file system storage operations.
#[async_trait(?Send)]
pub trait StoragePort: Send + Sync {
    /// Read a file and return its content as a string.
    async fn read_file(&self, path: &str) -> Result<String, VaultError>;

    /// Write content to a file (creates or overwrites).
    async fn write_file(&self, path: &str, content: &str) -> Result<(), VaultError>;

    /// Delete a file.
    async fn delete_file(&self, path: &str) -> Result<(), VaultError>;

    /// Create a directory (and parent directories if needed).
    async fn create_directory(&self, path: &str) -> Result<(), VaultError>;

    /// Delete a directory and all its contents recursively.
    async fn delete_directory(&self, path: &str) -> Result<(), VaultError>;

    /// Check if a directory exists.
    async fn directory_exists(&self, path: &str) -> Result<bool, VaultError>;

    /// List all entries (files and directories) in a directory.
    async fn list_entries(&self, path: &str) -> Result<Vec<String>, VaultError>;
}
