use std::sync::atomic::{AtomicBool, Ordering};

use wasm_bindgen_futures::JsFuture;

use crate::{errors::VaultError, global::get_storage_manager};

static PERSISTENCE_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn has_requested_persistence() -> bool {
    PERSISTENCE_REQUESTED.load(Ordering::Relaxed)
}

pub fn mark_persistence_requested() {
    PERSISTENCE_REQUESTED.store(true, Ordering::Relaxed);
}

pub async fn request_persistence_storage() -> Result<bool, VaultError> {
    mark_persistence_requested();

    let storage = get_storage_manager()?;

    let persist_promise = if let Ok(promise) = storage.persist() {
        promise
    } else {
        return Err(VaultError::JsError(
            "Unable to obtain a local storage shelf".to_string(),
        ));
    };

    let result = JsFuture::from(persist_promise).await?;

    Ok(result.as_bool().unwrap_or(false))
}

pub async fn check_storage_persistence() -> Result<bool, VaultError> {
    let storage = get_storage_manager()?;
    let persisted_promise = if let Ok(promise) = storage.persisted() {
        promise
    } else {
        return Err(VaultError::JsError(
            "Unable to obtain a local storage shelf".to_string(),
        ));
    };
    let result = JsFuture::from(persisted_promise).await?;
    let is_persisted = result.as_bool().unwrap_or(false);

    Ok(is_persisted)
}
