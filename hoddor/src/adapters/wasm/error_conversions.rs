use crate::domain::vault::error::VaultError;
use wasm_bindgen::JsValue;

impl From<JsValue> for VaultError {
    fn from(err: JsValue) -> Self {
        VaultError::io_error(
            err.as_string()
                .unwrap_or_else(|| "Unknown JavaScript error".to_string()),
        )
    }
}

impl From<VaultError> for JsValue {
    fn from(error: VaultError) -> Self {
        JsValue::from_str(&error.to_string())
    }
}
