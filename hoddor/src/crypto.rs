use crate::measure::time_it;
use argon2::Argon2;
use wasm_bindgen::prelude::JsValue;

pub fn derive_key(password: &[u8], salt: &[u8]) -> Result<[u8; 32], JsValue> {
    time_it!("derive_key", {
        let argon2 = Argon2::default();
        let mut key = [0u8; 32];

        argon2
            .hash_password_into(password, salt, &mut key)
            .map_err(|e| JsValue::from_str(&format!("Key derivation failed: {:?}", e)))?;

        Ok(key)
    })
}
