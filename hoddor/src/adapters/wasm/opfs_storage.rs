use async_trait::async_trait;
use crate::errors::VaultError;
use crate::global::get_storage_manager;
use crate::ports::StoragePort;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions};

/// OPFS storage adapter using File System Access API.
#[derive(Clone, Copy)]
pub struct OPFSStorage;

impl OPFSStorage {
    pub fn new() -> Self {
        Self
    }

    /// Get the root directory handle.
    async fn get_root(&self) -> Result<FileSystemDirectoryHandle, VaultError> {
        let storage = get_storage_manager()?;
        let dir_promise = storage.get_directory();
        let dir_handle = JsFuture::from(dir_promise)
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to get root directory",
            })?
            .unchecked_into::<FileSystemDirectoryHandle>();
        Ok(dir_handle)
    }

    /// Navigate to a directory from a path (e.g. "vault1" or "vault1/subdir").
    async fn navigate_to_dir(&self, path: &str) -> Result<FileSystemDirectoryHandle, VaultError> {
        let mut current = self.get_root().await?;

        if path.is_empty() || path == "." {
            return Ok(current);
        }

        for segment in path.split('/').filter(|s| !s.is_empty()) {
            current = JsFuture::from(current.get_directory_handle(segment))
                .await
                .map_err(|_| VaultError::IoError {
                    message: "Failed to navigate to directory",
                })?
                .unchecked_into::<FileSystemDirectoryHandle>();
        }

        Ok(current)
    }

    /// Split a path into (parent_dir, filename).
    fn split_path(path: &str) -> (&str, &str) {
        if let Some(pos) = path.rfind('/') {
            (&path[..pos], &path[pos + 1..])
        } else {
            (".", path)
        }
    }
}

#[async_trait(?Send)]
impl StoragePort for OPFSStorage {
    async fn read_file(&self, path: &str) -> Result<String, VaultError> {
        let (dir_path, filename) = Self::split_path(path);
        let dir_handle = self.navigate_to_dir(dir_path).await?;

        let file_handle = JsFuture::from(dir_handle.get_file_handle(filename))
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to get file handle",
            })?
            .unchecked_into::<FileSystemFileHandle>();

        let file = JsFuture::from(file_handle.get_file())
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to get file",
            })?;

        let text = JsFuture::from(file.unchecked_into::<web_sys::File>().text())
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to read file content",
            })?
            .as_string()
            .ok_or(VaultError::IoError {
                message: "Failed to convert file content to string",
            })?;

        Ok(text)
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), VaultError> {
        let (dir_path, filename) = Self::split_path(path);
        let dir_handle = self.navigate_to_dir(dir_path).await?;

        let options = FileSystemGetFileOptions::new();
        options.set_create(true);

        let file_handle = JsFuture::from(dir_handle.get_file_handle_with_options(filename, &options))
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to get or create file handle",
            })?
            .unchecked_into::<FileSystemFileHandle>();

        let writer = JsFuture::from(file_handle.create_writable())
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to create writable",
            })?;

        let promise = writer
            .unchecked_ref::<web_sys::FileSystemWritableFileStream>()
            .write_with_str(content)
            .map_err(|_| VaultError::IoError {
                message: "Failed to create write promise",
            })?;

        JsFuture::from(promise)
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to write file",
            })?;

        JsFuture::from(
            writer
                .unchecked_ref::<web_sys::FileSystemWritableFileStream>()
                .close(),
        )
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to close writer",
        })?;

        Ok(())
    }

    async fn delete_file(&self, path: &str) -> Result<(), VaultError> {
        let (dir_path, filename) = Self::split_path(path);
        let dir_handle = self.navigate_to_dir(dir_path).await?;

        JsFuture::from(dir_handle.remove_entry(filename))
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to delete file",
            })?;

        Ok(())
    }

    async fn create_directory(&self, path: &str) -> Result<(), VaultError> {
        let mut current = self.get_root().await?;

        if path.is_empty() || path == "." {
            return Ok(());
        }

        for segment in path.split('/').filter(|s| !s.is_empty()) {
            let options = web_sys::FileSystemGetDirectoryOptions::new();
            options.set_create(true);

            current = JsFuture::from(current.get_directory_handle_with_options(segment, &options))
                .await
                .map_err(|_| VaultError::IoError {
                    message: "Failed to create directory",
                })?
                .unchecked_into::<FileSystemDirectoryHandle>();
        }

        Ok(())
    }

    async fn delete_directory(&self, path: &str) -> Result<(), VaultError> {
        let (parent_path, dir_name) = Self::split_path(path);
        let parent_handle = self.navigate_to_dir(parent_path).await?;

        // Get the directory handle to clean it up first
        if let Ok(dir_handle) = JsFuture::from(parent_handle.get_directory_handle(dir_name))
            .await
            .map(|h| h.unchecked_into::<FileSystemDirectoryHandle>())
        {
            // Clean up all contents
            self.cleanup_directory(&dir_handle).await?;
        }

        // Remove the directory itself
        JsFuture::from(parent_handle.remove_entry(dir_name))
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to remove directory",
            })?;

        Ok(())
    }

    async fn directory_exists(&self, path: &str) -> Result<bool, VaultError> {
        match self.navigate_to_dir(path).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn list_entries(&self, path: &str) -> Result<Vec<String>, VaultError> {
        let dir_handle = self.navigate_to_dir(path).await?;
        let mut entries = Vec::new();

        let entries_val = js_sys::Reflect::get(&dir_handle, &JsValue::from_str("entries"))
            .map_err(|_| VaultError::IoError {
                message: "Failed to get entries",
            })?;

        let entries_fn = entries_val
            .dyn_ref::<js_sys::Function>()
            .ok_or_else(|| VaultError::IoError {
                message: "entries is not a function",
            })?;

        let iterator = entries_fn.call0(&dir_handle).map_err(|_| VaultError::IoError {
            message: "Failed to call entries",
        })?;

        loop {
            let next_val = js_sys::Reflect::get(&iterator, &JsValue::from_str("next"))
                .map_err(|_| VaultError::IoError {
                    message: "Failed to get next",
                })?;

            let next_fn = next_val
                .dyn_ref::<js_sys::Function>()
                .ok_or_else(|| VaultError::IoError {
                    message: "next is not a function",
                })?;

            let next_result = JsFuture::from(
                next_fn
                    .call0(&iterator)
                    .map_err(|_| VaultError::IoError {
                        message: "Failed to call next",
                    })?
                    .dyn_into::<js_sys::Promise>()
                    .map_err(|_| VaultError::IoError {
                        message: "Failed to convert to promise",
                    })?,
            )
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to await next",
            })?;

            let done = js_sys::Reflect::get(&next_result, &JsValue::from_str("done"))
                .map_err(|_| VaultError::IoError {
                    message: "Failed to get done status",
                })?
                .as_bool()
                .unwrap_or(true);

            if done {
                break;
            }

            if let Ok(value) = js_sys::Reflect::get(&next_result, &JsValue::from_str("value")) {
                if let Some(array) = value.dyn_ref::<js_sys::Array>() {
                    if let Some(name) = array.get(0).as_string() {
                        entries.push(name);
                    }
                }
            }
        }

        Ok(entries)
    }
}

impl OPFSStorage {
    /// Helper to clean up all files in a directory.
    async fn cleanup_directory(&self, dir_handle: &FileSystemDirectoryHandle) -> Result<(), VaultError> {
        let entries = self.list_entries_from_handle(dir_handle).await?;

        for entry_name in entries {
            JsFuture::from(dir_handle.remove_entry(&entry_name))
                .await
                .map_err(|_| VaultError::IoError {
                    message: "Failed to remove entry",
                })?;
        }

        Ok(())
    }

    /// Helper to list entries from a directory handle.
    async fn list_entries_from_handle(&self, dir_handle: &FileSystemDirectoryHandle) -> Result<Vec<String>, VaultError> {
        let mut entries = Vec::new();

        let entries_val = js_sys::Reflect::get(dir_handle, &JsValue::from_str("entries"))
            .map_err(|_| VaultError::IoError {
                message: "Failed to get entries",
            })?;

        let entries_fn = entries_val
            .dyn_ref::<js_sys::Function>()
            .ok_or_else(|| VaultError::IoError {
                message: "entries is not a function",
            })?;

        let iterator = entries_fn.call0(dir_handle).map_err(|_| VaultError::IoError {
            message: "Failed to call entries",
        })?;

        loop {
            let next_val = js_sys::Reflect::get(&iterator, &JsValue::from_str("next"))
                .map_err(|_| VaultError::IoError {
                    message: "Failed to get next",
                })?;

            let next_fn = next_val
                .dyn_ref::<js_sys::Function>()
                .ok_or_else(|| VaultError::IoError {
                    message: "next is not a function",
                })?;

            let next_result = JsFuture::from(
                next_fn
                    .call0(&iterator)
                    .map_err(|_| VaultError::IoError {
                        message: "Failed to call next",
                    })?
                    .dyn_into::<js_sys::Promise>()
                    .map_err(|_| VaultError::IoError {
                        message: "Failed to convert to promise",
                    })?,
            )
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to await next",
            })?;

            let done = js_sys::Reflect::get(&next_result, &JsValue::from_str("done"))
                .map_err(|_| VaultError::IoError {
                    message: "Failed to get done status",
                })?
                .as_bool()
                .unwrap_or(true);

            if done {
                break;
            }

            if let Ok(value) = js_sys::Reflect::get(&next_result, &JsValue::from_str("value")) {
                if let Some(array) = value.dyn_ref::<js_sys::Array>() {
                    if let Some(name) = array.get(0).as_string() {
                        entries.push(name);
                    }
                }
            }
        }

        Ok(entries)
    }
}
