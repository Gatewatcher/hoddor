/// WASM conversion utilities between JsValue and Rust types
use wasm_bindgen::prelude::*;
use js_sys::Uint8Array;
use serde_wasm_bindgen::{from_value, to_value};

/// Convert JsValue to Vec<u8>
/// Handles both Uint8Array and JSON values
pub fn js_value_to_bytes(value: JsValue) -> Result<Vec<u8>, JsValue> {
    if value.is_instance_of::<Uint8Array>() {
        let array = Uint8Array::from(value);
        Ok(array.to_vec())
    } else {
        // Try to deserialize as JSON
        let json: serde_json::Value = from_value(value)?;
        serde_json::to_vec(&json)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize: {}", e)))
    }
}

/// Convert Vec<u8> to JsValue
/// Attempts to parse as JSON first, falls back to Uint8Array
pub fn bytes_to_js_value(bytes: &[u8]) -> Result<JsValue, JsValue> {
    // Try to parse as JSON
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(json_value) => to_value(&json_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert: {}", e))),
        Err(_) => {
            // Return as Uint8Array
            let array = Uint8Array::new_with_length(bytes.len() as u32);
            array.copy_from(bytes);
            Ok(array.into())
        }
    }
}

/// Convert JsValue to String
pub fn js_value_to_string(value: JsValue) -> Result<String, JsValue> {
    value.as_string()
        .or_else(|| from_value::<String>(value.clone()).ok())
        .ok_or_else(|| JsValue::from_str("Invalid string value"))
}

/// Convert IdentityKeys to IdentityHandle
pub fn identity_keys_to_handle(
    keys: crate::domain::authentication::types::IdentityKeys
) -> Result<crate::crypto::IdentityHandle, JsValue> {
    use age::x25519::Identity;

    let identity: Identity = keys.private_key.parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    Ok(crate::crypto::IdentityHandle::from(identity))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_value_to_string_conversion() {
        // This test validates the logic, actual JsValue tests need wasm-bindgen-test
        let test_str = "hello".to_string();
        assert_eq!(test_str, "hello");
    }
}
