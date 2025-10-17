use crate::domain::vault::error::VaultError;
use crate::global::get_global_scope;
use crate::ports::{LockGuard, LockPort};
use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Lock, LockManager, LockOptions, WorkerGlobalScope};

pub struct WebLockGuard {
    _lock: Lock,
    _callback: Closure<dyn Fn()>,
}

impl LockGuard for WebLockGuard {}

impl Drop for WebLockGuard {
    fn drop(&mut self) {}
}

#[derive(Clone, Copy)]
pub struct Locks;

impl Locks {
    pub fn new() -> Self {
        Self
    }

    async fn get_lock_manager(&self) -> Result<LockManager, VaultError> {
        let global = get_global_scope()?;

        if let Ok(worker) = global.clone().dyn_into::<WorkerGlobalScope>() {
            Ok(worker.navigator().locks())
        } else if let Ok(window) = global.dyn_into::<web_sys::Window>() {
            Ok(window.navigator().locks())
        } else {
            Err(VaultError::io_error("Could not access navigator"))
        }
    }
}

#[async_trait(?Send)]
impl LockPort for Locks {
    async fn acquire(&self, name: &str) -> Result<Box<dyn LockGuard>, VaultError> {
        let lock_manager = self.get_lock_manager().await?;
        let lock_name = format!("vault_{}_lock", name);
        let mut retries = 10;
        let mut delay = 50;

        while retries > 0 {
            let options = LockOptions::new();
            js_sys::Reflect::set(
                &options,
                &JsValue::from_str("mode"),
                &JsValue::from_str("exclusive"),
            )?;
            options.set_if_available(true);

            let callback = Closure::wrap(Box::new(|| {}) as Box<dyn Fn()>);
            let promise = lock_manager.request_with_options_and_callback(
                &lock_name,
                &options,
                callback.as_ref().unchecked_ref(),
            );

            match JsFuture::from(promise).await {
                Ok(lock) => {
                    let web_lock = lock.unchecked_into::<Lock>();
                    let guard = WebLockGuard {
                        _lock: web_lock,
                        _callback: callback,
                    };
                    return Ok(Box::new(guard));
                }
                Err(_) => {
                    retries -= 1;
                    if retries > 0 {
                        delay = ((delay as f64 * 1.5) as u32).min(1000);
                        let jitter = (js_sys::Math::random() * 50.0) as u32;
                        gloo_timers::future::TimeoutFuture::new(delay + jitter).await;
                    }
                }
            }
        }

        Err(VaultError::io_error("Failed to acquire lock"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_locks_creation() {
        let _locks = Locks::new();
    }

    #[wasm_bindgen_test]
    async fn test_acquire_single_lock() {
        let locks = Locks::new();
        let guard = locks.acquire("test_lock").await;
        assert!(guard.is_ok(), "Should acquire lock successfully");
    }

    #[wasm_bindgen_test]
    async fn test_lock_guard_drop() {
        let locks = Locks::new();

        {
            let _guard = locks.acquire("test_drop").await.unwrap();
        }

        let guard2 = locks.acquire("test_drop").await;
        assert!(
            guard2.is_ok(),
            "Should be able to acquire lock after previous guard dropped"
        );
    }

    #[wasm_bindgen_test]
    async fn test_multiple_different_locks() {
        let locks = Locks::new();

        let guard1 = locks.acquire("lock_1").await;
        assert!(guard1.is_ok(), "Should acquire first lock");

        let guard2 = locks.acquire("lock_2").await;
        assert!(
            guard2.is_ok(),
            "Should acquire second lock with different name"
        );

        let guard3 = locks.acquire("lock_3").await;
        assert!(
            guard3.is_ok(),
            "Should acquire third lock with different name"
        );
    }

    #[wasm_bindgen_test]
    async fn test_sequential_same_lock() {
        let locks = Locks::new();

        let guard1 = locks.acquire("sequential").await;
        assert!(guard1.is_ok(), "First acquisition should succeed");
        drop(guard1);

        let guard2 = locks.acquire("sequential").await;
        assert!(
            guard2.is_ok(),
            "Second acquisition should succeed after first release"
        );
        drop(guard2);

        let guard3 = locks.acquire("sequential").await;
        assert!(guard3.is_ok(), "Third acquisition should succeed");
    }

    #[wasm_bindgen_test]
    async fn test_lock_name_formatting() {
        let locks = Locks::new();

        let guard_a = locks.acquire("vault_a").await;
        assert!(guard_a.is_ok(), "Should acquire lock for vault_a");

        let guard_b = locks.acquire("vault_b").await;
        assert!(guard_b.is_ok(), "Should acquire lock for vault_b");
    }

    #[wasm_bindgen_test]
    async fn test_get_lock_manager() {
        let locks = Locks::new();
        let lock_manager = locks.get_lock_manager().await;
        assert!(lock_manager.is_ok(), "Should be able to get lock manager");
    }
}
