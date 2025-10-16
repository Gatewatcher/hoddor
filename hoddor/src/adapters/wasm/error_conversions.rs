use crate::domain::vault::error::VaultError;
use wasm_bindgen::JsValue;

/// Conversion from JsValue to VaultError for WASM infrastructure
impl From<JsValue> for VaultError {
    fn from(err: JsValue) -> Self {
        VaultError::io_error(
            err.as_string()
                .unwrap_or_else(|| "Unknown JavaScript error".to_string())
        )
    }
}
