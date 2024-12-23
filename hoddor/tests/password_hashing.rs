#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use hoddor::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_password_hashing() {
    let password: JsValue = "test_password123".into();
    let hash = hash_password(password.clone()).expect("Failed to hash password");

    assert!(!hash.is_null(), "Hash should not be null/undefined");
    assert!(!hash.is_undefined(), "Hash should not be null/undefined");

    let hash_str = hash.as_string().unwrap();
    assert_ne!(
        hash_str,
        password.as_string().unwrap(),
        "Hash should be different from password"
    );
}

#[wasm_bindgen_test]
fn test_hash_uniqueness() {
    let password: JsValue = "same_password".into();
    let hash1 = hash_password(password.clone()).expect("Failed to create first hash");
    let hash2 = hash_password(password).expect("Failed to create second hash");

    assert_ne!(
        hash1.as_string().unwrap(),
        hash2.as_string().unwrap(),
        "Hashes should be unique even for the same password"
    );
}

#[wasm_bindgen_test]
fn test_empty_password() {
    let empty_password: JsValue = "".into();
    let hash = hash_password(empty_password).expect("Failed to hash empty password");

    assert!(
        !hash.is_null(),
        "Empty password should still produce a hash"
    );
    assert!(
        !hash.is_undefined(),
        "Empty password should still produce a hash"
    );
}

#[wasm_bindgen_test]
fn test_long_password() {
    let long_password: JsValue = "a".repeat(1000).into();
    let hash = hash_password(long_password.clone()).expect("Failed to hash long password");

    assert!(!hash.is_null(), "Long password hash should not be null");
    assert!(
        !hash.is_undefined(),
        "Long password hash should not be undefined"
    );
    assert_ne!(
        hash.as_string().unwrap(),
        long_password.as_string().unwrap(),
        "Long password hash should be different from password"
    );
}

#[wasm_bindgen_test]
fn test_unicode_password() {
    let unicode_password: JsValue = "パスワード123!@#".into();
    let hash = hash_password(unicode_password.clone()).expect("Failed to hash unicode password");

    assert!(!hash.is_null(), "Unicode password hash should not be null");
    assert!(
        !hash.is_undefined(),
        "Unicode password hash should not be undefined"
    );
    assert_ne!(
        hash.as_string().unwrap(),
        unicode_password.as_string().unwrap(),
        "Unicode password hash should be different from password"
    );
}

#[wasm_bindgen_test]
fn test_whitespace_password() {
    let whitespace_password: JsValue = "   spaces   tabs\t\t\tnewlines\n\n\n   ".into();
    let hash =
        hash_password(whitespace_password.clone()).expect("Failed to hash whitespace password");

    assert!(
        !hash.is_null(),
        "Whitespace password hash should not be null"
    );
    assert!(
        !hash.is_undefined(),
        "Whitespace password hash should not be undefined"
    );
    assert_ne!(
        hash.as_string().unwrap(),
        whitespace_password.as_string().unwrap(),
        "Whitespace password hash should be different from password"
    );
}

#[wasm_bindgen_test]
fn test_null_character_password() {
    let null_char_password: JsValue = "pass\0word\0with\0nulls".into();
    let hash =
        hash_password(null_char_password.clone()).expect("Failed to hash null character password");

    assert!(
        !hash.is_null(),
        "Null character password hash should not be null"
    );
    assert!(
        !hash.is_undefined(),
        "Null character password hash should not be undefined"
    );
    assert_ne!(
        hash.as_string().unwrap(),
        null_char_password.as_string().unwrap(),
        "Null character password hash should be different from password"
    );
}

#[wasm_bindgen_test]
fn test_null_password() {
    let null_password: JsValue = JsValue::NULL;
    let result = hash_password(null_password);
    assert!(result.is_err(), "Hashing null password should fail");
}

#[wasm_bindgen_test]
fn test_undefined_password() {
    let undefined_password: JsValue = JsValue::UNDEFINED;
    let result = hash_password(undefined_password);
    assert!(result.is_err(), "Hashing undefined password should fail");
}

#[wasm_bindgen_test]
async fn test_concurrent_password_hashing() {
    let passwords: Vec<JsValue> = (0..3).map(|i| format!("password{}", i).into()).collect();

    let mut hash_futures = Vec::new();
    for password in passwords {
        let future = wasm_bindgen_futures::future_to_promise(async move {
            hash_password(password).map_err(|e| e.into())
        });
        hash_futures.push(future);
    }

    for future in hash_futures {
        wasm_bindgen_futures::JsFuture::from(future)
            .await
            .expect("Failed to hash password concurrently");
    }
}
