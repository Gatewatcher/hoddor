use async_trait::async_trait;
use crate::errors::VaultError;
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
        fs::read_to_string(&full_path).map_err(|_| VaultError::IoError {
            message: "Failed to read file",
        })
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), VaultError> {
        let full_path = self.get_full_path(path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|_| VaultError::IoError {
                message: "Failed to create parent directories",
            })?;
        }

        fs::write(&full_path, content).map_err(|_| VaultError::IoError {
            message: "Failed to write file",
        })
    }

    async fn delete_file(&self, path: &str) -> Result<(), VaultError> {
        let full_path = self.get_full_path(path);
        fs::remove_file(&full_path).map_err(|_| VaultError::IoError {
            message: "Failed to delete file",
        })
    }

    async fn create_directory(&self, path: &str) -> Result<(), VaultError> {
        let full_path = self.get_full_path(path);
        fs::create_dir_all(&full_path).map_err(|_| VaultError::IoError {
            message: "Failed to create directory",
        })
    }

    async fn delete_directory(&self, path: &str) -> Result<(), VaultError> {
        let full_path = self.get_full_path(path);
        fs::remove_dir_all(&full_path).map_err(|_| VaultError::IoError {
            message: "Failed to delete directory",
        })
    }

    async fn directory_exists(&self, path: &str) -> Result<bool, VaultError> {
        let full_path = self.get_full_path(path);
        Ok(full_path.exists() && full_path.is_dir())
    }

    async fn list_entries(&self, path: &str) -> Result<Vec<String>, VaultError> {
        let full_path = self.get_full_path(path);
        let entries = fs::read_dir(&full_path).map_err(|_| VaultError::IoError {
            message: "Failed to read directory",
        })?;

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
}
