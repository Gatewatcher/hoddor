#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use hoddor::crypto::identity_from_passphrase;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_identity_from_passphrase_basic() {
    let password = "test_password123";
    let salt = [0u8; 32];
    let identity = identity_from_passphrase(password, &salt).await.expect("Failed to derive identity");
    assert!(!identity.public_key().is_empty(), "Public key should not be empty");
    assert!(!identity.private_key().is_empty(), "Private key should not be empty");
}

#[wasm_bindgen_test]
async fn test_identity_from_passphrase_uniqueness() {
    let password = "same_password";
    let salt1 = [1u8; 32];
    let salt2 = [2u8; 32];
    
    let identity1 = identity_from_passphrase(password, &salt1).await.expect("Failed to derive identity1");
    let identity2 = identity_from_passphrase(password, &salt2).await.expect("Failed to derive identity2");
    
    assert_ne!(
        identity1.public_key(), identity2.public_key(),
        "Derived identities should be unique for different salts"
    );
}

#[wasm_bindgen_test]
async fn test_identity_from_passphrase_empty_password() {
    let empty_password = "";
    let salt = [3u8; 32];
    let identity = identity_from_passphrase(empty_password, &salt).await.expect("Failed to derive identity from empty password");
    assert!(!identity.public_key().is_empty(), "Public key should not be empty");
    assert!(!identity.private_key().is_empty(), "Private key should not be empty");
}

#[wasm_bindgen_test]
async fn test_identity_from_passphrase_long_password() {
    let long_password = &"a".repeat(1000);
    let salt = [4u8; 32];
    let identity = identity_from_passphrase(long_password, &salt).await.expect("Failed to derive identity from long password");
    assert!(!identity.public_key().is_empty(), "Public key should not be empty");
    assert!(!identity.private_key().is_empty(), "Private key should not be empty");
}

#[wasm_bindgen_test]
async fn test_identity_from_passphrase_unicode_password() {
    let unicode_password = "パスワード123!@#";
    let salt = [5u8; 32];
    let identity = identity_from_passphrase(unicode_password, &salt).await.expect("Failed to derive identity from unicode password");
    assert!(!identity.public_key().is_empty(), "Public key should not be empty");
    assert!(!identity.private_key().is_empty(), "Private key should not be empty");
}

#[wasm_bindgen_test]
async fn test_identity_from_passphrase_whitespace_password() {
    let whitespace_password = "   spaces   tabs\t\t\tnewlines\n\n\n   ";
    let salt = [6u8; 32];
    let identity = identity_from_passphrase(whitespace_password, &salt).await.expect("Failed to derive identity from whitespace password");
    assert!(!identity.public_key().is_empty(), "Public key should not be empty");
    assert!(!identity.private_key().is_empty(), "Private key should not be empty");
}

#[wasm_bindgen_test]
async fn test_identity_from_passphrase_null_character_password() {
    let null_char_password = "pass\0word\0with\0nulls";
    let salt = [7u8; 32];
    let identity = identity_from_passphrase(null_char_password, &salt).await.expect("Failed to derive identity with null characters");
    assert!(!identity.public_key().is_empty(), "Public key should not be empty");
    assert!(!identity.private_key().is_empty(), "Private key should not be empty");
}

#[wasm_bindgen_test]
async fn test_concurrent_identity_derivation() {
    let passwords = ["password0", "password1", "password2"];
    let salt = [9u8; 32];

    let mut derive_futures = Vec::new();
    for &password in &passwords {
        let password = password.to_string();
        let salt_clone = salt.clone();
        
        let future = wasm_bindgen_futures::spawn_local(async move {
            match identity_from_passphrase(&password, &salt_clone).await {
                Ok(identity) => {
                    // Verify the identity has valid keys
                    assert!(!identity.public_key().is_empty());
                    assert!(!identity.private_key().is_empty());
                }
                Err(e) => {
                    panic!("Failed to derive identity concurrently: {:?}", e);
                }
            }
        });
        
        derive_futures.push(future);
    }

    // No need to await the futures as they're already spawned
}

#[wasm_bindgen_test]
async fn test_identity_from_passphrase_salt_too_short() {
    let password = "short_salt_password";
    let salt = [10u8; 1]; // Too short salt

    let result = identity_from_passphrase(password, &salt).await;

    assert!(
        result.is_err(),
        "Expected an error with salt that's too short, but got success"
    );
}
