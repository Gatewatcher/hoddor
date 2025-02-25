use crate::errors::VaultError;
use wasm_bindgen::prelude::JsValue;
use wasm_bindgen::prelude::*;
use web_sys::{self, Window, WorkerGlobalScope};

pub fn get_global_scope() -> Result<JsValue, VaultError> {
    // Try worker scope first
    if let Ok(scope) = js_sys::global().dyn_into::<WorkerGlobalScope>() {
        return Ok(JsValue::from(scope));
    }

    // Fallback to window
    let window = web_sys::window().ok_or(VaultError::IoError {
        message: "Neither WorkerGlobalScope nor Window found",
    })?;
    Ok(JsValue::from(window))
}

pub fn window() -> Window {
    get_global_scope()
        .expect("Unable to retrieve global scope")
        .dyn_into::<Window>()
        .expect("Unable to retrieve window")
}
