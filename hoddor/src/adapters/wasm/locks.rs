use async_trait::async_trait;
use crate::errors::{LockError, VaultError};
use crate::global::get_global_scope;
use crate::ports::{LockGuard, LockPort};
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
            Err(VaultError::IoError {
                message: "Could not access navigator",
            })
        }
    }
}

#[async_trait(?Send)]
impl LockPort for Locks {
    async fn acquire(&self, name: &str) -> Result<Box<dyn LockGuard>, VaultError> {
        let lock_manager = self.get_lock_manager().await?;
        let lock_name = format!("vault_{}_lock", name);
        let mut retries = 10;
        let mut delay = 50; // Start with 50ms delay

        while retries > 0 {
            let options = LockOptions::new();
            js_sys::Reflect::set(
                &options,
                &JsValue::from_str("mode"),
                &JsValue::from_str("exclusive"),
            )?;
            options.set_if_available(true); // Try to acquire if available

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
                        // Exponential backoff with jitter
                        delay = ((delay as f64 * 1.5) as u32).min(1000);
                        let jitter = (js_sys::Math::random() * 50.0) as u32;
                        gloo_timers::future::TimeoutFuture::new(delay + jitter).await;
                    }
                }
            }
        }

        Err(LockError::AcquisitionFailed.into())
    }
}
