use super::converters;
use super::crypto::IdentityHandle;
use crate::domain::vault::{operations, validation};
use crate::platform::Platform;
use std::sync::atomic::{AtomicI64, Ordering};
use wasm_bindgen::prelude::*;

static CLEANUP_INTERVAL: AtomicI64 = AtomicI64::new(0);
static LAST_CLEANUP: AtomicI64 = AtomicI64::new(0);

#[wasm_bindgen]
pub async fn vault_identity_from_passphrase(
    passphrase: &str,
    vault_name: &str,
) -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();

    validation::validate_passphrase(passphrase).map_err(converters::to_js_error)?;
    validation::validate_vault_name(vault_name)?;

    let mut vault = operations::read_vault(&platform, vault_name)
        .await
        .map_err(|e| {
            converters::to_js_error(format!("Vault '{}' does not exist: {}", vault_name, e))
        })?;

    let identity_keys = crate::domain::authentication::derive_vault_identity(
        &platform, passphrase, vault_name, &mut vault,
    )
    .await
    .map_err(converters::to_js_error)?;

    operations::save_vault(&platform, vault_name, vault).await?;

    converters::identity_keys_to_handle(identity_keys)
}

#[wasm_bindgen]
pub async fn upsert_vault(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: &str,
    data: JsValue,
    expires_in_seconds: Option<i64>,
    replace_if_exists: bool,
) -> Result<(), JsValue> {
    let platform = Platform::new();

    validation::validate_namespace(namespace).map_err(converters::to_js_error)?;

    let data_bytes = converters::js_value_to_bytes(data)?;

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

#[wasm_bindgen]
pub async fn read_from_vault(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    let namespace_str = converters::js_value_to_string(namespace)?;

    validation::validate_namespace(&namespace_str).map_err(converters::to_js_error)?;

    let data_bytes = operations::read_namespace(
        &platform,
        vault_name,
        &identity.private_key(),
        &namespace_str,
    )
    .await
    .map_err(converters::to_js_error)?;

    converters::bytes_to_js_value(&data_bytes)
}

#[wasm_bindgen]
pub async fn remove_from_vault(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<(), JsValue> {
    let platform = Platform::new();

    let namespace_str = converters::js_value_to_string(namespace)?;
    validation::validate_namespace(&namespace_str).map_err(converters::to_js_error)?;

    operations::verify_vault_identity(&platform, vault_name, &identity.private_key()).await?;

    operations::remove_namespace(&platform, vault_name, &namespace_str)
        .await
        .map_err(|e| e.into())
}

#[wasm_bindgen]
pub async fn list_namespaces(vault_name: &str) -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    let namespaces = operations::list_namespaces_in_vault(&platform, vault_name)
        .await
        .map_err(converters::to_js_error)?;

    converters::to_js_value(&namespaces)
}

#[wasm_bindgen]
pub async fn create_vault(vault_name: JsValue) -> Result<(), JsValue> {
    let platform = Platform::new();

    let name = vault_name
        .as_string()
        .ok_or_else(|| JsValue::from_str("vault_name must be a string"))?;

    validation::validate_vault_name(&name).map_err(converters::to_js_error)?;

    if operations::read_vault(&platform, &name).await.is_ok() {
        return Err(JsValue::from_str(&format!(
            "Vault '{}' already exists",
            name
        )));
    }

    let vault = operations::create_vault().await?;

    operations::save_vault(&platform, &name, vault)
        .await
        .map_err(|e| e.into())
}

#[wasm_bindgen]
pub async fn remove_vault(vault_name: &str) -> Result<(), JsValue> {
    let platform = Platform::new();

    operations::delete_vault(&platform, vault_name)
        .await
        .map_err(|e| e.into())
}

#[wasm_bindgen]
pub async fn list_vaults() -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    let vaults = operations::list_vaults(&platform)
        .await
        .map_err(converters::to_js_error)?;

    converters::to_js_value(&vaults)
}

#[wasm_bindgen]
pub async fn export_vault(vault_name: &str) -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    let vault_bytes = operations::export_vault_bytes(&platform, vault_name)
        .await
        .map_err(converters::to_js_error)?;

    let array = js_sys::Uint8Array::new_with_length(vault_bytes.len() as u32);
    array.copy_from(&vault_bytes);
    Ok(array.into())
}

#[wasm_bindgen]
pub async fn import_vault(vault_name: &str, data: JsValue) -> Result<(), JsValue> {
    let platform = Platform::new();

    let vault_bytes = converters::js_value_to_bytes(data)?;

    operations::import_vault_from_bytes(&platform, vault_name, &vault_bytes)
        .await
        .map_err(|e| e.into())
}

#[wasm_bindgen]
pub async fn force_cleanup_vault(vault_name: &str) -> Result<(), JsValue> {
    let platform = Platform::new();

    loop {
        let data_removed = operations::cleanup_vault(&platform, vault_name)
            .await
            .map_err(converters::to_js_error)?;

        if !data_removed {
            break;
        }
    }

    Ok(())
}

#[wasm_bindgen]
pub fn configure_cleanup(interval_seconds: i64) {
    if interval_seconds > 0 {
        web_sys::console::log_1(
            &format!(
                "Configuring cleanup with interval of {} seconds",
                interval_seconds
            )
            .into(),
        );
        CLEANUP_INTERVAL.store(interval_seconds, Ordering::SeqCst);
        LAST_CLEANUP.store(js_sys::Date::now() as i64 / 1000, Ordering::SeqCst);
    } else {
        web_sys::console::log_1(&"Disabling automatic cleanup".into());
        CLEANUP_INTERVAL.store(0, Ordering::SeqCst);
    }
}
