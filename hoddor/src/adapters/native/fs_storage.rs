use async_trait::async_trait;
use crate::domain::vault::error::VaultError;
use crate::ports::StoragePort;
use std::fs;
use std::path::PathBuf;

/// Native filesystem storage adapter using std::fs.
#[derive(Clone, Copy)]
pub struct FsStorage {
    root_path: &'static str,
}

impl FsStorage {
    pub fn new() -> Self {
        Self {
            root_path: "./hoddor_data",
        }
    }

    /// Get the full path by joining root with the relative path.
    fn get_full_path(&self, path: &str) -> PathBuf {
        if path.is_empty() || path == "." {
            PathBuf::from(self.root_path)
        } else {
            PathBuf::from(self.root_path).join(path)
        }
    }
}

#[async_trait(?Send)]
impl StoragePort for FsStorage {
    async fn read_file(&self, path: &str) -> Result<String, VaultError> {
        let full_path = self.get_full_path(path);
        fs::read_to_string(&full_path).map_err(|_| VaultError::io_error("Failed to read file"))
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), VaultError> {
        let full_path = self.get_full_path(path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|_| VaultError::io_error("Failed to create parent directories"))?;
        }

        fs::write(&full_path, content).map_err(|_| VaultError::io_error("Failed to write file"))
    }

    async fn delete_file(&self, path: &str) -> Result<(), VaultError> {
        let full_path = self.get_full_path(path);
        fs::remove_file(&full_path).map_err(|_| VaultError::io_error("Failed to delete file"))
    }

    async fn create_directory(&self, path: &str) -> Result<(), VaultError> {
        let full_path = self.get_full_path(path);
        fs::create_dir_all(&full_path).map_err(|_| VaultError::io_error("Failed to create directory"))
    }

    async fn delete_directory(&self, path: &str) -> Result<(), VaultError> {
        let full_path = self.get_full_path(path);
        fs::remove_dir_all(&full_path).map_err(|_| VaultError::io_error("Failed to delete directory"))
    }

    async fn directory_exists(&self, path: &str) -> Result<bool, VaultError> {
        let full_path = self.get_full_path(path);
        Ok(full_path.exists() && full_path.is_dir())
    }

    async fn list_entries(&self, path: &str) -> Result<Vec<String>, VaultError> {
        let full_path = self.get_full_path(path);
        let entries = fs::read_dir(&full_path).map_err(|_| VaultError::io_error("Failed to read directory"))?;

        let mut names = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }

        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_storage_creation() {
        let _storage = FsStorage::new();
    }

    #[test]
    fn test_get_full_path() {
        let storage = FsStorage::new();
        let path = storage.get_full_path("vault1/metadata.json");
        assert!(path.to_str().unwrap().contains("vault1/metadata.json"));
    }

    #[test]
    fn test_get_full_path_root() {
        let storage = FsStorage::new();
        let path = storage.get_full_path(".");
        assert_eq!(path.to_str().unwrap(), "./hoddor_data");
    }

    #[test]
    fn test_file_lifecycle() {
        use futures::executor::block_on;
        let storage = FsStorage::new();
        let test_dir = "test_lifecycle";
        let test_file = "test_lifecycle/test.txt";
        let content = "test content";

        block_on(async {
            storage.create_directory(test_dir).await.unwrap();
            assert!(storage.directory_exists(test_dir).await.unwrap());

            storage.write_file(test_file, content).await.unwrap();
            let read_content = storage.read_file(test_file).await.unwrap();
            assert_eq!(read_content, content);

            storage.delete_file(test_file).await.unwrap();
            storage.delete_directory(test_dir).await.unwrap();
            assert!(!storage.directory_exists(test_dir).await.unwrap());
        });
    }

    #[test]
    fn test_list_entries() {
        use futures::executor::block_on;
        let storage = FsStorage::new();
        let test_dir = "test_list";

        block_on(async {
            storage.create_directory(test_dir).await.unwrap();
            storage.write_file("test_list/file1.txt", "content1").await.unwrap();
            storage.write_file("test_list/file2.txt", "content2").await.unwrap();
            storage.write_file("test_list/file3.txt", "content3").await.unwrap();

            let entries = storage.list_entries(test_dir).await.unwrap();
            assert_eq!(entries.len(), 3);
            assert!(entries.contains(&"file1.txt".to_string()));
            assert!(entries.contains(&"file2.txt".to_string()));
            assert!(entries.contains(&"file3.txt".to_string()));

            storage.delete_directory(test_dir).await.unwrap();
        });
    }

    #[test]
    fn test_delete_directory_with_contents() {
        use futures::executor::block_on;
        let storage = FsStorage::new();
        let test_dir = "test_delete";

        block_on(async {
            storage.create_directory(test_dir).await.unwrap();
            storage.write_file("test_delete/file1.txt", "content1").await.unwrap();
            storage.write_file("test_delete/file2.txt", "content2").await.unwrap();
            storage.create_directory("test_delete/subdir").await.unwrap();
            storage.write_file("test_delete/subdir/file3.txt", "content3").await.unwrap();

            storage.delete_directory(test_dir).await.unwrap();
            assert!(!storage.directory_exists(test_dir).await.unwrap());
        });
    }

    #[test]
    fn test_directory_exists() {
        use futures::executor::block_on;
        let storage = FsStorage::new();
        let test_dir = "test_exists";

        block_on(async {
            assert!(!storage.directory_exists(test_dir).await.unwrap());

            storage.create_directory(test_dir).await.unwrap();
            assert!(storage.directory_exists(test_dir).await.unwrap());

            storage.delete_directory(test_dir).await.unwrap();
            assert!(!storage.directory_exists(test_dir).await.unwrap());
        });
    }
}
