use async_trait::async_trait;
use crate::errors::VaultError;
use crate::global::get_storage_manager;
use crate::ports::PersistencePort;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen_futures::JsFuture;

static PERSISTENCE_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
pub struct Persistence;

impl Persistence {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait(?Send)]
impl PersistencePort for Persistence {
    fn has_requested(&self) -> bool {
        PERSISTENCE_REQUESTED.load(Ordering::Relaxed)
    }

    async fn request(&self) -> Result<bool, VaultError> {
        PERSISTENCE_REQUESTED.store(true, Ordering::Relaxed);

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

    async fn check(&self) -> Result<bool, VaultError> {
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
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_persistence_creation() {
        let persistence = Persistence::new();
        let _ = persistence.has_requested(); // Smoke test
    }

    #[wasm_bindgen_test]
    async fn test_persistence_check() {
        let persistence = Persistence::new();
        // Just verify we can call check without panic
        let _ = persistence.check().await;
    }

    #[wasm_bindgen_test]
    async fn test_persistence_request() {
        let persistence = Persistence::new();
        // Just verify we can call request without panic
        let _ = persistence.request().await;
    }
}
