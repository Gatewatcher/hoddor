/// WASM facade for vault operations
/// This module provides JavaScript-compatible functions that delegate to domain logic
use wasm_bindgen::prelude::*;
use super::crypto::IdentityHandle;
use crate::platform::Platform;
use crate::domain::vault::{operations, validation};
use super::converters;

/// Derive identity from passphrase for a vault (WASM facade)
#[wasm_bindgen]
pub async fn vault_identity_from_passphrase_v2(
    passphrase: &str,
    vault_name: &str,
) -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();

    // Validate inputs
    validation::validate_passphrase(passphrase)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    validation::validate_vault_name(vault_name)?;

    // Read vault
    let mut vault = operations::read_vault(&platform, vault_name)
        .await
        .map_err(|e| JsValue::from_str(&format!("Vault '{}' does not exist: {}", vault_name, e)))?;

    // Derive identity using domain authentication
    let identity_keys = crate::domain::authentication::derive_vault_identity(
        &platform,
        passphrase,
        vault_name,
        &mut vault,
    )
    .await
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Save vault with new salt if created
    operations::save_vault(&platform, vault_name, vault).await?;

    // Convert to IdentityHandle
    converters::identity_keys_to_handle(identity_keys)
}

/// Insert or update data in a vault namespace (WASM facade)
/// Temporary name during migration - will replace upsert_vault
#[wasm_bindgen]
pub async fn upsert_vault_v2(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: &str,
    data: JsValue,
    expires_in_seconds: Option<i64>,
    replace_if_exists: bool,
) -> Result<(), JsValue> {
    let platform = Platform::new();

    // Validate namespace
    validation::validate_namespace(namespace)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Convert WASM → Rust
    let data_bytes = converters::js_value_to_bytes(data)?;

    // Call domain logic (pure function)
    operations::upsert_namespace(
        &platform,
        vault_name,
        &identity.public_key(),
        namespace,
        data_bytes,
        expires_in_seconds,
        replace_if_exists,
    )
    .await
    .map_err(|e| e.into())
}

/// Read and decrypt data from a vault namespace (WASM facade)
#[wasm_bindgen]
pub async fn read_from_vault_v2(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    // Convert namespace
    let namespace_str = converters::js_value_to_string(namespace)?;

    // Validate namespace
    validation::validate_namespace(&namespace_str)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Call domain logic
    let data_bytes = operations::read_namespace(
        &platform,
        vault_name,
        &identity.private_key(),
        &namespace_str,
    )
    .await
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Convert Rust → WASM
    converters::bytes_to_js_value(&data_bytes)
}

/// Remove a namespace from a vault (WASM facade)
#[wasm_bindgen]
pub async fn remove_from_vault_v2(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<(), JsValue> {
    let platform = Platform::new();

    // Convert and validate namespace
    let namespace_str = converters::js_value_to_string(namespace)?;
    validation::validate_namespace(&namespace_str)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Verify identity has access
    operations::verify_vault_identity(
        &platform,
        vault_name,
        &identity.private_key(),
    )
    .await?;

    // Remove namespace
    operations::remove_namespace(&platform, vault_name, &namespace_str)
        .await
        .map_err(|e| e.into())
}

/// List all namespaces in a vault (WASM facade)
#[wasm_bindgen]
pub async fn list_namespaces_v2(vault_name: &str) -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    let namespaces = operations::list_namespaces_in_vault(&platform, vault_name)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&namespaces)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Create a new vault (WASM facade)
#[wasm_bindgen]
pub async fn create_vault_v2(vault_name: JsValue) -> Result<(), JsValue> {
    let platform = Platform::new();

    // Convert vault name
    let name = vault_name
        .as_string()
        .ok_or_else(|| JsValue::from_str("vault_name must be a string"))?;

    // Validate vault name
    validation::validate_vault_name(&name)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Check if vault already exists
    if operations::read_vault(&platform, &name).await.is_ok() {
        return Err(JsValue::from_str(&format!("Vault '{}' already exists", name)));
    }

    // Create vault
    let vault = operations::create_vault().await?;

    // Save vault
    operations::save_vault(&platform, &name, vault)
        .await
        .map_err(|e| e.into())
}

/// Remove a vault (WASM facade)
#[wasm_bindgen]
pub async fn remove_vault_v2(vault_name: &str) -> Result<(), JsValue> {
    let platform = Platform::new();

    operations::delete_vault(&platform, vault_name)
        .await
        .map_err(|e| e.into())
}

/// List all vaults (WASM facade)
#[wasm_bindgen]
pub async fn list_vaults_v2() -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    let vaults = operations::list_vaults(&platform)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&vaults)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Export a vault as bytes (WASM facade)
#[wasm_bindgen]
pub async fn export_vault_v2(vault_name: &str) -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    let vault_bytes = operations::export_vault_bytes(&platform, vault_name)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Convert to Uint8Array
    let array = js_sys::Uint8Array::new_with_length(vault_bytes.len() as u32);
    array.copy_from(&vault_bytes);
    Ok(array.into())
}

/// Import a vault from bytes (WASM facade)
#[wasm_bindgen]
pub async fn import_vault_v2(vault_name: &str, data: JsValue) -> Result<(), JsValue> {
    let platform = Platform::new();

    // Convert data to bytes
    let vault_bytes = if data.is_instance_of::<js_sys::Uint8Array>() {
        let array = js_sys::Uint8Array::from(data);
        array.to_vec()
    } else {
        serde_wasm_bindgen::from_value(data)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert input data: {:?}", e)))?
    };

    // Import vault
    operations::import_vault_from_bytes(&platform, vault_name, &vault_bytes)
        .await
        .map_err(|e| e.into())
}

/// Force cleanup of expired data in a vault (WASM facade)
#[wasm_bindgen]
pub async fn force_cleanup_vault_v2(vault_name: &str) -> Result<(), JsValue> {
    let platform = Platform::new();

    // Run cleanup until no more expired data
    loop {
        let data_removed = operations::cleanup_vault(&platform, vault_name)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        if !data_removed {
            break;
        }
    }

    Ok(())
}
