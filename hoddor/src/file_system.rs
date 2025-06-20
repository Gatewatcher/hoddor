use crate::errors::VaultError;
use crate::{console::log, global::get_storage_manager};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use web_sys::{self, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions};

pub async fn get_root_directory_handle() -> Result<FileSystemDirectoryHandle, VaultError> {
    let storage = get_storage_manager()?;

    let dir_promise = storage.get_directory();
    let dir_handle = JsFuture::from(dir_promise)
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to get directory",
        })?
        .unchecked_into::<FileSystemDirectoryHandle>();

    Ok(dir_handle)
}

pub async fn get_or_create_file_handle_with_name(
    filename: &str,
) -> Result<FileSystemFileHandle, VaultError> {
    let root = get_root_directory_handle().await?;
    let options = FileSystemGetFileOptions::new();
    options.set_create(true);

    let file_handle = JsFuture::from(root.get_file_handle_with_options(filename, &options))
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to get or create file handle",
        })?
        .unchecked_into::<FileSystemFileHandle>();

    Ok(file_handle)
}

pub async fn get_or_create_directory_handle(
    dirname: &str,
) -> Result<FileSystemDirectoryHandle, VaultError> {
    let root = get_root_directory_handle().await?;
    let options = web_sys::FileSystemGetDirectoryOptions::new();
    options.set_create(true);

    let dir_handle = JsFuture::from(root.get_directory_handle_with_options(dirname, &options))
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to get or create directory handle",
        })?
        .unchecked_into::<FileSystemDirectoryHandle>();

    Ok(dir_handle)
}

pub async fn get_or_create_file_handle_in_directory(
    dir_handle: &FileSystemDirectoryHandle,
    filename: &str,
) -> Result<FileSystemFileHandle, VaultError> {
    let options = FileSystemGetFileOptions::new();
    options.set_create(true);

    let file_handle = JsFuture::from(dir_handle.get_file_handle_with_options(filename, &options))
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to get or create file handle in directory",
        })?
        .unchecked_into::<FileSystemFileHandle>();

    Ok(file_handle)
}

pub async fn cleanup_directory(dir_handle: &FileSystemDirectoryHandle) -> Result<(), VaultError> {
    if let Ok(entries_val) = js_sys::Reflect::get(dir_handle, &JsValue::from_str("entries")) {
        if let Some(entries_fn) = entries_val.dyn_ref::<js_sys::Function>() {
            if let Ok(iterator) = entries_fn.call0(dir_handle) {
                loop {
                    let next_val = js_sys::Reflect::get(&iterator, &JsValue::from_str("next"))
                        .map_err(|_| VaultError::IoError {
                            message: "Failed to get next entry",
                        })?;

                    if let Some(next_fn) = next_val.dyn_ref::<js_sys::Function>() {
                        if let Ok(promise) = next_fn.call0(&iterator) {
                            let next_result = JsFuture::from(
                                promise.dyn_into::<js_sys::Promise>().map_err(|_| {
                                    VaultError::IoError {
                                        message: "Failed to get promise for next entry",
                                    }
                                })?,
                            )
                            .await
                            .map_err(|_| VaultError::IoError {
                                message: "Failed to await next entry",
                            })?;

                            let done =
                                js_sys::Reflect::get(&next_result, &JsValue::from_str("done"))
                                    .unwrap_or(JsValue::TRUE)
                                    .as_bool()
                                    .unwrap_or(true);

                            if done {
                                break;
                            }

                            if let Ok(value) =
                                js_sys::Reflect::get(&next_result, &JsValue::from_str("value"))
                                    .and_then(|v| v.dyn_into::<js_sys::Array>())
                            {
                                if let Some(name) = value.get(0).as_string() {
                                    remove_file_from_directory(dir_handle, &name).await?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn remove_directory_with_contents(
    root: &FileSystemDirectoryHandle,
    dir_name: &str,
) -> Result<(), VaultError> {
    log(&format!("Attempting to remove directory: {}", dir_name));

    if let Ok(dir_handle) = JsFuture::from(root.get_directory_handle(dir_name))
        .await
        .map(|h| h.unchecked_into::<FileSystemDirectoryHandle>())
    {
        if let Err(e) = cleanup_directory(&dir_handle).await {
            log(&format!("Error cleaning up directory contents: {:?}", e));
            return Err(e);
        }
    }

    JsFuture::from(root.remove_entry(dir_name))
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to remove directory",
        })?;

    Ok(())
}

pub async fn remove_file_from_directory(
    dir_handle: &FileSystemDirectoryHandle,
    filename: &str,
) -> Result<(), VaultError> {
    log(&format!("Attempting to remove file: {}", filename));

    JsFuture::from(dir_handle.remove_entry(filename))
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to remove file",
        })?;

    Ok(())
}
