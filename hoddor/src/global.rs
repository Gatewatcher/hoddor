use crate::errors::VaultError;
use wasm_bindgen::prelude::JsValue;
use wasm_bindgen::prelude::*;
use web_sys::{self, DedicatedWorkerGlobalScope, StorageManager, Window, WorkerGlobalScope};

pub fn get_global_scope() -> Result<JsValue, VaultError> {
    // Try worker scope first
    if let Ok(scope) = js_sys::global().dyn_into::<DedicatedWorkerGlobalScope>() {
        return Ok(JsValue::from(scope));
    }

    // Fallback to window
    let window = web_sys::window().ok_or(VaultError::IoError {
        message: "Neither DedicatedWorkerGlobalScope nor Window found",
    })?;
    Ok(JsValue::from(window))
}

pub fn window() -> Window {
    get_global_scope()
        .expect("Unable to retrieve global scope")
        .dyn_into::<Window>()
        .expect("Unable to retrieve window")
}

pub fn get_storage_manager() -> Result<StorageManager, VaultError> {
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

    Ok(storage)
}
