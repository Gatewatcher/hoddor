use crate::domain::vault::error::VaultError;
use crate::global::get_storage_manager;
use crate::ports::PersistencePort;
use async_trait::async_trait;
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
            return Err(VaultError::io_error(
                "Unable to obtain a local storage shelf",
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
            return Err(VaultError::io_error(
                "Unable to obtain a local storage shelf",
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
        let _persistence = Persistence::new();
    }

    #[wasm_bindgen_test]
    async fn test_persistence_check_returns_bool() {
        let persistence = Persistence::new();
        let result = persistence.check().await;

        assert!(result.is_ok(), "check() should return Ok");
    }

    #[wasm_bindgen_test]
    async fn test_persistence_request_returns_bool() {
        let persistence = Persistence::new();
        let result = persistence.request().await;

        assert!(result.is_ok(), "request() should return Ok");
    }

    #[wasm_bindgen_test]
    async fn test_has_requested_state_change() {
        let persistence = Persistence::new();

        let _ = persistence.request().await;
        assert!(
            persistence.has_requested(),
            "has_requested should return true after request()"
        );
    }

    #[wasm_bindgen_test]
    async fn test_multiple_instances_share_state() {
        let persistence1 = Persistence::new();
        let persistence2 = Persistence::new();

        let _ = persistence1.request().await;

        assert!(
            persistence2.has_requested(),
            "Multiple instances should share persistence state"
        );
    }

    #[wasm_bindgen_test]
    async fn test_sequential_check_calls() {
        let persistence = Persistence::new();

        let result1 = persistence.check().await;
        assert!(result1.is_ok(), "First check should succeed");

        let result2 = persistence.check().await;
        assert!(result2.is_ok(), "Second check should succeed");
    }

    #[wasm_bindgen_test]
    async fn test_request_then_check() {
        let persistence = Persistence::new();

        let request_result = persistence.request().await;
        assert!(request_result.is_ok(), "request() should succeed");

        let check_result = persistence.check().await;
        assert!(
            check_result.is_ok(),
            "check() should succeed after request()"
        );
    }
}
