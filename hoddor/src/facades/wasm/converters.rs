use js_sys::Uint8Array;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

pub fn js_value_to_bytes(value: JsValue) -> Result<Vec<u8>, JsValue> {
    if value.is_instance_of::<Uint8Array>() {
        let array = Uint8Array::from(value);
        Ok(array.to_vec())
    } else {
        let json: serde_json::Value = from_value(value)?;
        serde_json::to_vec(&json)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize: {}", e)))
    }
}

pub fn bytes_to_js_value(bytes: &[u8]) -> Result<JsValue, JsValue> {
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(json_value) => to_value(&json_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert: {}", e))),
        Err(_) => {
            let array = Uint8Array::new_with_length(bytes.len() as u32);
            array.copy_from(bytes);
            Ok(array.into())
        }
    }
}

pub fn to_js_error<E: std::fmt::Display>(error: E) -> JsValue {
    JsValue::from_str(&error.to_string())
}

pub fn to_js_value<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
    to_value(value).map_err(to_js_error)
}

pub fn js_value_to_string(value: JsValue) -> Result<String, JsValue> {
    value
        .as_string()
        .or_else(|| from_value::<String>(value.clone()).ok())
        .ok_or_else(|| JsValue::from_str("Invalid string value"))
}

pub fn identity_keys_to_handle(
    keys: crate::domain::authentication::types::IdentityKeys,
) -> Result<super::crypto::IdentityHandle, JsValue> {
    use age::x25519::Identity;

    let identity: Identity = keys
        .private_key
        .parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    Ok(super::crypto::IdentityHandle::from(identity))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_js_value_to_string_conversion() {
        let test_str = "hello".to_string();
        assert_eq!(test_str, "hello");
    }
}
