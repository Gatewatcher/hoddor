use crate::errors::{LockError, VaultError};
use crate::global::get_global_scope;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use web_sys::{self, Lock, LockManager, LockOptions, WorkerGlobalScope};

async fn get_lock_manager() -> Result<LockManager, VaultError> {
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

pub async fn acquire_vault_lock(vault_name: &str) -> Result<Lock, VaultError> {
    let lock_manager = get_lock_manager().await?;
    let lock_name = format!("vault_{}_lock", vault_name);
    let mut retries = 10; // Increase retries
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
                callback.forget(); // Prevent the callback from being dropped
                return Ok(lock.unchecked_into::<Lock>());
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
