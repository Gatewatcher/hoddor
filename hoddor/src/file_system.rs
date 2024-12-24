use crate::errors::VaultError;
use crate::global::get_global_scope;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use web_sys::{
    self, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    WorkerGlobalScope,
};

pub async fn get_root_directory_handle() -> Result<FileSystemDirectoryHandle, VaultError> {
    let global = get_global_scope()?;

    let storage = if let Ok(worker) = global.clone().dyn_into::<WorkerGlobalScope>() {
        worker.navigator().storage()
    } else if let Ok(window) = global.dyn_into::<web_sys::Window>() {
        window.navigator().storage()
    } else {
        return Err(VaultError::IoError {
            message: "Could not access navigator",
        });
    };

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
