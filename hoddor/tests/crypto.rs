#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use hoddor::crypto::derive_key;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen_futures::{future_to_promise, JsFuture};

wasm_bindgen_test_configure!(run_in_browser);

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

// Helper to convert a JsValue password to a byte slice.
fn get_password_bytes(password: JsValue) -> Result<Vec<u8>, JsValue> {
    password
        .as_string()
        .map(|s| s.into_bytes())
        .ok_or_else(|| JsValue::from_str("Invalid or non-string password"))
}

#[wasm_bindgen_test]
fn test_derive_key_basic() {
    let password = JsValue::from_str("test_password123");
    let pass_bytes = get_password_bytes(password).expect("Failed to retrieve password bytes");
    let key = derive_key(&pass_bytes, b"default_salt").expect("Failed to derive key");
    assert_eq!(key.len(), 32, "Derived key should be 32 bytes long");
}

#[wasm_bindgen_test]
fn test_derive_key_uniqueness() {
    let password = JsValue::from_str("same_password");
    let pass_bytes = get_password_bytes(password.clone()).expect("Failed to retrieve password bytes");
    let key1 = derive_key(&pass_bytes, b"default_salt1").expect("Failed to derive key1");
    let key2 = derive_key(&pass_bytes, b"default_salt2").expect("Failed to derive key2");
    assert_ne!(
        key1, key2,
        "Derived keys should be unique for different salts"
    );
}

#[wasm_bindgen_test]
fn test_derive_key_empty_password() {
    let empty_password = JsValue::from_str("");
    let pass_bytes = get_password_bytes(empty_password).expect("Failed to retrieve password bytes");
    let key = derive_key(&pass_bytes, b"any_salt")
        .expect("Failed to derive key from empty password");
    assert_eq!(key.len(), 32, "Derived key should still be 32 bytes long");
}

#[wasm_bindgen_test]
fn test_derive_key_long_password() {
    let long_password = JsValue::from_str(&"a".repeat(1000));
    let pass_bytes = get_password_bytes(long_password.clone()).expect("Failed to retrieve password bytes");
    let key = derive_key(&pass_bytes, b"long_salt")
        .expect("Failed to derive key from long password");
    assert_eq!(key.len(), 32, "Derived key should be 32 bytes");
}

#[wasm_bindgen_test]
fn test_derive_key_unicode_password() {
    let unicode_password = JsValue::from_str("パスワード123!@#");
    let pass_bytes = get_password_bytes(unicode_password.clone()).expect("Failed to retrieve password bytes");
    let key = derive_key(&pass_bytes, b"unicode_salt")
        .expect("Failed to derive key from unicode password");
    assert_eq!(key.len(), 32, "Derived key should be 32 bytes long");
}

#[wasm_bindgen_test]
fn test_derive_key_whitespace_password() {
    let whitespace_password = JsValue::from_str("   spaces   tabs\t\t\tnewlines\n\n\n   ");
    let pass_bytes = get_password_bytes(whitespace_password.clone()).expect("Failed to retrieve password bytes");
    let key = derive_key(&pass_bytes, b"salt_with_whitespace")
        .expect("Failed to derive key");
    assert_eq!(key.len(), 32, "Derived key should be 32 bytes long");
}

#[wasm_bindgen_test]
fn test_derive_key_null_character_password() {
    let null_char_password = JsValue::from_str("pass\0word\0with\0nulls");
    let pass_bytes = get_password_bytes(null_char_password.clone()).expect("Failed to retrieve password bytes");
    let key = derive_key(&pass_bytes, b"null_salt")
        .expect("Failed to derive key with null characters");
    assert_eq!(key.len(), 32, "Derived key should be 32 bytes long");
}

#[wasm_bindgen_test]
fn test_derive_key_null_password() {
    let null_password = JsValue::NULL;
    let result = get_password_bytes(null_password).and_then(|p| derive_key(&p, b"some_salt"));
    assert!(
        result.is_err(),
        "Deriving key from null password should fail"
    );
}

#[wasm_bindgen_test]
fn test_derive_key_undefined_password() {
    let undefined_password = JsValue::UNDEFINED;
    let result = get_password_bytes(undefined_password).and_then(|p| derive_key(&p, b"some_salt"));
    assert!(
        result.is_err(),
        "Deriving key from undefined password should fail"
    );
}

#[wasm_bindgen_test]
async fn test_concurrent_key_derivation() {
    let passwords: Vec<JsValue> = (0..3).map(|i| format!("password{}", i).into()).collect();

    let mut derive_futures = Vec::new();
    for password in passwords {
        let future = future_to_promise(async move {
            let pass_bytes = get_password_bytes(password)?;
            let key = derive_key(&pass_bytes, b"concurrent_salt").map_err(|e| e)?;
            // Convert derived key into a hex-like string for returning as JsValue
            Ok(JsValue::from_str(&bytes_to_hex(&key)))
        });
        derive_futures.push(future);
    }

    for future in derive_futures {
        JsFuture::from(future)
            .await
            .expect("Failed to derive key concurrently");
    }
}
#[wasm_bindgen_test]
fn test_derive_key_salt_too_short() {
    let password = JsValue::from_str("short_salt_password");
    let pass_bytes = get_password_bytes(password).expect("Failed to retrieve password bytes");

    let result = derive_key(&pass_bytes, b"a");
    
    match result {
        Ok(_) => panic!("Expected SaltTooShort error, but derive_key succeeded"),
        Err(e) => {
            let err_str = e.as_string().unwrap_or_default();
            assert!(
                err_str.contains("SaltTooShort"),
                "Expected an error mentioning 'SaltTooShort', got: {}",
                err_str
            );
        }
    }
}
