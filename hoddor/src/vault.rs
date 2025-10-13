use crate::crypto::{
    decrypt_with_identity, encrypt_with_recipients, identity_from_passphrase, IdentityHandle,
};
use crate::errors::VaultError;
use crate::file_system::{
    get_or_create_directory_handle, get_or_create_file_handle_in_directory,
    get_root_directory_handle, remove_directory_with_contents, remove_file_from_directory,
};
use crate::global::get_global_scope;
use crate::lock::acquire_vault_lock;
use crate::measure::get_performance;
use crate::measure::time_it;
use crate::persistence::{
    check_storage_persistence, has_requested_persistence, request_persistence_storage,
};
use crate::sync::{get_sync_manager, OperationType, SyncMessage};
use crate::webrtc::{AccessLevel, WebRtcPeer};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::adapters::logger;
use core::str;
use serde_wasm_bindgen::{from_value, to_value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use wasm_bindgen_futures::JsFuture;

use web_sys::{self, FileSystemDirectoryHandle};

use rand::RngCore;

use argon2::password_hash::rand_core::OsRng;

use futures_channel::mpsc::UnboundedReceiver;

use crate::notifications;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Expiration {
    expires_at: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct NamespaceData {
    data: Vec<u8>,
    expiration: Option<Expiration>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct VaultMetadata {
    pub peer_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct IdentitySalts {
    salts: HashMap<String, [u8; 32]>,
    credential_ids: HashMap<String, Vec<u8>>,
}

impl Default for IdentitySalts {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentitySalts {
    pub fn new() -> Self {
        Self {
            salts: HashMap::new(),
            credential_ids: HashMap::new(),
        }
    }

    pub fn get_salt(&self, public_key: &str) -> Option<&[u8; 32]> {
        self.salts.get(public_key)
    }

    pub fn set_salt(&mut self, public_key: String, salt: [u8; 32]) {
        self.salts.insert(public_key, salt);
    }

    pub fn get_all_salts(&self) -> impl Iterator<Item = &[u8; 32]> {
        self.salts.values()
    }

    pub fn get_all_credential_ids(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.credential_ids.values()
    }

    pub fn get_credential_id(&self, public_key: &str) -> Option<&Vec<u8>> {
        self.credential_ids.get(public_key)
    }

    pub fn set_credential_id(&mut self, public_key: String, credential_id: Vec<u8>) {
        self.credential_ids.insert(public_key, credential_id);
    }

    pub fn get_public_keys_with_credentials(&self) -> impl Iterator<Item = &String> {
        self.credential_ids.keys()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Vault {
    pub metadata: VaultMetadata,
    pub identity_salts: IdentitySalts,
    pub username_pk: HashMap<String, String>,
    pub namespaces: HashMap<String, NamespaceData>,
    pub sync_enabled: bool,
}

pub fn get_vault_dirname(vault_name: &str) -> String {
    vault_name.to_string()
}

fn get_metadata_filename() -> String {
    "metadata.json".to_string()
}

fn get_namespace_filename(namespace: &str) -> String {
    format!("{}.ns", namespace)
}

fn validate_namespace(namespace: &str) -> Result<(), VaultError> {
    if namespace.trim().is_empty() {
        return Err(VaultError::IoError {
            message: "Namespace cannot be empty or whitespace only",
        });
    }

    let invalid_chars = ['/', '\\', '<', '>', ':', '"', '|', '?', '*'];
    if namespace.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(VaultError::IoError {
            message: "Namespace contains invalid characters",
        });
    }
    Ok(())
}

fn validate_passphrase(passphrase: &str) -> Result<(), VaultError> {
    if passphrase.trim().is_empty() {
        return Err(VaultError::JsError(
            "Passphrase cannot be empty or whitespace".to_string(),
        ));
    }
    Ok(())
}

#[wasm_bindgen]
pub async fn vault_identity_from_passphrase(
    passphrase: &str,
    vault_name: &str,
) -> Result<IdentityHandle, JsValue> {
    validate_passphrase(passphrase).map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    validate_vault_name(vault_name)?;

    let dirname = get_vault_dirname(vault_name);
    let (dir_handle, mut vault) = match read_vault_with_name(&dirname).await {
        Ok(result) => result,
        Err(_) => {
            return Err(JsValue::from_str(&format!(
                "Vault '{}' does not exist",
                vault_name
            )));
        }
    };

    // Try to find an existing identity by iterating over stored salts
    for (stored_pubkey, salt) in &vault.identity_salts.salts {
        logger().log(&format!("Checking stored public key: {}", stored_pubkey));

        // Validate salt length
        if salt.len() != 32 {
            logger().error(&format!(
                "Invalid salt length ({}) for public key: {}",
                salt.len(),
                stored_pubkey
            ));
            continue;
        }

        logger().log(&format!("Using salt: {:?}", salt));

        match identity_from_passphrase(passphrase, salt).await {
            Ok(identity) => {
                logger().log(&format!("Generated public key: {}", identity.public_key()));
                if identity.public_key() == *stored_pubkey {
                    logger().log("Found matching identity");
                    return Ok(identity);
                } else {
                    logger().warn("Public key does not match stored salt");
                }
            }
            Err(err) => {
                logger().warn(&format!(
                    "Failed to generate identity with stored salt for public key {}: {:?}",
                    stored_pubkey, err
                ));
            }
        };
    }

    logger().log("No matching identity found; generating new salt");
    let mut new_salt = [0u8; 32];
    OsRng.fill_bytes(&mut new_salt);

    let identity = identity_from_passphrase(passphrase, &new_salt)
        .await
        .map_err(|e| {
            logger().error(&format!("Failed to create new identity: {:?}", e));
            JsValue::from_str(&format!("Failed to create new identity: {:?}", e))
        })?;

    // Store the new salt with the generated public key
    vault
        .identity_salts
        .set_salt(identity.public_key(), new_salt);

    save_vault(&dir_handle, vault).await.map_err(|e| {
        logger().error(&format!("Failed to save vault: {:?}", e));
        JsValue::from_str(&format!("Failed to save vault: {:?}", e))
    })?;

    Ok(identity)
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
    let mut retries = 10;
    let mut delay = 50;
    let mut last_error = None;

    while retries > 0 {
        let lock = match acquire_vault_lock(vault_name).await {
            Ok(lock) => lock,
            Err(e) => {
                retries -= 1;
                if retries > 0 {
                    // Exponential backoff with jitter
                    delay = ((delay as f64 * 1.5) as u32).min(1000);
                    let jitter = (js_sys::Math::random() * 50.0) as u32;
                    gloo_timers::future::TimeoutFuture::new(delay + jitter).await;
                    continue;
                } else {
                    return Err(e.into());
                }
            }
        };

        let result = async {
            let mut vault = match read_vault_with_name(vault_name).await {
                Ok((_, v)) => v,
                Err(VaultError::IoError { message })
                    if message.contains("Failed to get directory handle") =>
                {
                    create_vault_internal(vault_name).await?;
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };

            if vault.namespaces.contains_key(namespace) && !replace_if_exists {
                return Err(VaultError::NamespaceAlreadyExists.into());
            }

            let namespace_data =
                insert_namespace_data(identity, data.clone(), expires_in_seconds).await?;
            vault
                .namespaces
                .insert(namespace.to_string(), namespace_data.clone());

            let (dir_handle, _) = read_vault_with_name(vault_name).await?;
            save_vault(&dir_handle, vault.clone()).await?;

            if let Ok(sync_manager) = get_sync_manager(vault_name) {
                let mut sync_manager = sync_manager.borrow_mut();

                let operation = sync_manager.create_operation(
                    namespace.to_string(),
                    OperationType::Update,
                    Some(namespace_data.data),
                    None,
                );

                let sync_msg = sync_manager.create_sync_message(
                    vault_name.to_string(),
                    operation,
                    Some(vault.metadata.clone()),
                    Some(vault.identity_salts.clone()),
                    Some(vault.username_pk.clone()),
                );

                let peers = sync_manager.get_peers_mut();
                for peer in peers.values() {
                    let peer = peer.borrow();
                    if peer.is_ready() {
                        let msg_data = serde_json::to_vec(&sync_msg).map_err(|e| {
                            JsValue::from_str(&format!("Failed to serialize sync message: {}", e))
                        })?;
                        peer.send_message(msg_data).map_err(|e| {
                            JsValue::from_str(&format!("Failed to send sync message: {:?}", e))
                        })?;
                    }
                }
            }

            Ok(())
        }
        .await;

        drop(lock);

        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e);
                retries -= 1;
                if retries > 0 {
                    delay = ((delay as f64 * 1.5) as u32).min(1000);
                    let jitter = (js_sys::Math::random() * 50.0) as u32;
                    gloo_timers::future::TimeoutFuture::new(delay + jitter).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        VaultError::IoError {
            message: "Failed to access vault after retries",
        }
        .into()
    }))
}

#[wasm_bindgen]
pub async fn remove_from_vault(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<(), JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    let namespace: String = from_value(namespace)?;

    check_identity(vault_name, identity).await?;

    let (dir_handle, mut vault) = read_vault_with_name(vault_name).await?;

    if vault.namespaces.remove(&namespace).is_none() {
        return Err(VaultError::NamespaceNotFound.into());
    }

    let namespace_filename = get_namespace_filename(&namespace);
    remove_file_from_directory(&dir_handle, &namespace_filename).await?;

    save_vault(&dir_handle, vault).await?;
    Ok(())
}

#[wasm_bindgen]
pub async fn read_from_vault(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<JsValue, JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    let namespace_str = namespace.as_string().unwrap_or_default();
    if get_performance().is_some() {
        logger().time(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    let result = time_it!("Total read_from_vault", {
        let namespace: String = from_value(namespace).map_err(|_| VaultError::IoError {
            message: "Invalid namespace format",
        })?;

        let (dir_handle, mut vault) = match read_vault_with_name(vault_name).await {
            Ok(result) => result,
            Err(VaultError::IoError { .. }) => {
                return Err(VaultError::NamespaceNotFound.into());
            }
            Err(e) => return Err(e.into()),
        };

        let encrypted_namespace = vault
            .namespaces
            .get(&namespace)
            .ok_or(VaultError::NamespaceNotFound)?;

        if let Some(expiration) = &encrypted_namespace.expiration {
            let now = js_sys::Date::now() as i64 / 1000;
            if now >= expiration.expires_at {
                vault.namespaces.remove(&namespace);
                save_vault(&dir_handle, vault.clone()).await?;
                return Err(VaultError::DataExpired.into());
            }
        }

        let decrypted_data = time_it!("Decryption", {
            decrypt_with_identity(&encrypted_namespace.data, identity)
                .await
                .map_err(|_| VaultError::InvalidPassword)?
        });

        time_it!("JSON conversion", {
            // Attempt to parse as JSON.
            match serde_json::from_slice::<serde_json::Value>(&decrypted_data) {
                Ok(json_value) => to_value(&json_value).map_err(|_| {
                    VaultError::SerializationError {
                        message: "Failed to convert JSON to JS value",
                    }
                    .into()
                }),
                Err(_) => {
                    // If not JSON, return the raw bytes as a Uint8Array.
                    to_value(&decrypted_data).map_err(|_| {
                        VaultError::SerializationError {
                            message: "Failed to convert bytes to JS value",
                        }
                        .into()
                    })
                }
            }
        })
    });

    if get_performance().is_some() {
        logger().time_end(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    result
}

#[wasm_bindgen]
pub async fn list_namespaces(vault_name: &str) -> Result<JsValue, JsValue> {
    let (_, vault) = match read_vault_with_name(vault_name).await {
        Ok(result) => result,
        Err(VaultError::IoError { .. }) => {
            return Ok(to_value(&Vec::<String>::new())?);
        }
        Err(e) => return Err(e.into()),
    };

    logger().log(&format!(
        "Found {} namespaces in vault",
        vault.namespaces.len()
    ));
    for key in vault.namespaces.keys() {
        logger().log(&format!("Namespace found: {}", key));
    }

    let namespaces: Vec<String> = vault.namespaces.keys().cloned().collect();

    logger().log(&format!("Returning {} namespaces", namespaces.len()));
    Ok(to_value(&namespaces)?)
}

#[wasm_bindgen]
pub async fn remove_vault(vault_name: &str) -> Result<(), JsValue> {
    let _lock = acquire_vault_lock(vault_name).await?;

    let dirname = get_vault_dirname(vault_name);
    let root_handle = get_root_directory_handle().await?;
    remove_directory_with_contents(&root_handle, &dirname).await?;

    Ok(())
}

#[wasm_bindgen]
pub async fn list_vaults() -> Result<JsValue, JsValue> {
    let root = get_root_directory_handle().await?;
    logger().log("Listing vaults from root directory");

    let entries_val = js_sys::Reflect::get(&root, &JsValue::from_str("entries"))?;
    let entries_fn = entries_val
        .dyn_ref::<js_sys::Function>()
        .ok_or_else(|| JsValue::from_str("entries is not a function"))?;

    let iterator = entries_fn.call0(&root)?;
    let mut vault_names = Vec::new();

    loop {
        let next_val = js_sys::Reflect::get(&iterator, &JsValue::from_str("next"))?;
        let next_fn = next_val
            .dyn_ref::<js_sys::Function>()
            .ok_or_else(|| JsValue::from_str("next is not a function"))?;

        let next_result =
            JsFuture::from(next_fn.call0(&iterator)?.dyn_into::<js_sys::Promise>()?).await?;

        let done = js_sys::Reflect::get(&next_result, &JsValue::from_str("done"))?
            .as_bool()
            .unwrap_or(true);

        if done {
            break;
        }

        if next_result.is_null() {
            break;
        }

        if let Ok(value) = js_sys::Reflect::get(&next_result, &JsValue::from_str("value")) {
            if let Some(array) = value.dyn_ref::<js_sys::Array>() {
                if let Some(name) = array.get(0).as_string() {
                    vault_names.push(name);
                }
            }
        }
    }

    logger().log(&format!("Found {} vaults in total", vault_names.len()));
    Ok(to_value(&vault_names)?)
}

fn validate_vault_name(name: &str) -> Result<(), VaultError> {
    if name.trim().is_empty() {
        return Err(VaultError::IoError {
            message: "Vault name cannot be empty or whitespace only",
        });
    }
    if name.contains(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-') {
        return Err(VaultError::IoError {
            message:
                "Vault name can only contain alphanumeric characters, underscores, and hyphens",
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub async fn create_vault(vault_name: JsValue) -> Result<(), JsValue> {
    let name = vault_name
        .as_string()
        .ok_or_else(|| JsValue::from_str("vault_name must be a string"))?;

    create_vault_internal(&name).await
}

async fn create_vault_internal(vault_name: &str) -> Result<(), JsValue> {
    let dirname = get_vault_dirname(vault_name);

    validate_vault_name(vault_name).map_err(|e| JsValue::from_str(&format!("{}", e)))?;

    if let Ok(_) = read_vault_with_name(vault_name).await {
        return Err(JsValue::from_str(&format!(
            "Vault '{}' already exists",
            vault_name
        )));
    }

    let dir_handle = get_or_create_directory_handle(&dirname)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create directory: {}", e)))?;

    let vault = Vault {
        metadata: VaultMetadata { peer_id: None },
        identity_salts: IdentitySalts::new(),
        username_pk: HashMap::new(),
        namespaces: HashMap::new(),
        sync_enabled: false,
    };

    save_vault(&dir_handle, vault)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to save vault: {:?}", e)))?;

    Ok(())
}

async fn check_identity(vault_name: &str, identity: &IdentityHandle) -> Result<Vault, VaultError> {
    let (_, vault) = match read_vault_with_name(vault_name).await {
        Ok((handle, existing_vault)) => (handle, existing_vault),

        Err(VaultError::IoError { .. }) => {
            return Err(VaultError::VaultNotFound);
        }

        Err(e) => return Err(e),
    };

    if let Some((_, first_encrypted)) = vault.namespaces.iter().next() {
        decrypt_with_identity(&first_encrypted.data, identity)
            .await
            .map_err(|_| VaultError::InvalidPassword)?;
    }
    Ok(vault)
}

#[wasm_bindgen]
pub async fn export_vault(vault_name: &str) -> Result<JsValue, JsValue> {
    let vault = read_vault_with_name(vault_name).await?.1;

    // Create binary format with magic number "VAULT1"
    let magic = b"VAULT1";
    let serialized = serde_json::to_vec(&vault).map_err(|e| {
        logger().log(&format!("Serialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to serialize vault for export",
        }
    })?;

    let total_size = magic.len() + 4 + serialized.len();
    let mut vault_bytes = Vec::with_capacity(total_size);

    vault_bytes.extend_from_slice(magic);
    vault_bytes.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
    vault_bytes.extend_from_slice(&serialized);

    logger().log(&format!(
        "Exporting vault data: {} bytes (magic: {}, length: 4, content: {})",
        vault_bytes.len(),
        magic.len(),
        serialized.len()
    ));

    let array = js_sys::Uint8Array::new_with_length(vault_bytes.len() as u32);
    array.copy_from(&vault_bytes);

    Ok(array.into())
}

#[wasm_bindgen]
pub async fn import_vault(vault_name: &str, data: JsValue) -> Result<(), JsValue> {
    let vault_bytes = if data.is_instance_of::<js_sys::Uint8Array>() {
        let array = js_sys::Uint8Array::from(data);
        array.to_vec()
    } else {
        from_value(data)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert input data: {:?}", e)))?
    };

    logger().log(&format!(
        "Attempting to import vault data of size: {} bytes",
        vault_bytes.len()
    ));

    if vault_bytes.len() < 10 || &vault_bytes[..6] != b"VAULT1" {
        return Err(VaultError::SerializationError {
            message: "Invalid vault file: missing or incorrect magic number",
        }
        .into());
    }

    let length = u32::from_be_bytes([
        vault_bytes[6],
        vault_bytes[7],
        vault_bytes[8],
        vault_bytes[9],
    ]) as usize;

    if vault_bytes.len() != length + 10 {
        return Err(VaultError::SerializationError {
            message: "Invalid vault file: content length mismatch",
        }
        .into());
    }

    let imported_vault: Vault = serde_json::from_slice(&vault_bytes[10..]).map_err(|e| {
        logger().log(&format!("Deserialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to deserialize vault data",
        }
    })?;

    match read_vault_with_name(vault_name).await {
        Ok(_) => {
            return Err(VaultError::VaultAlreadyExists.into());
        }
        Err(VaultError::IoError { .. }) => {
            logger().log(&format!(
                "No existing vault named '{}'; proceeding with import.",
                vault_name
            ));
        }
        Err(e) => {
            return Err(e.into());
        }
    };

    let dir_handle = get_or_create_directory_handle(&get_vault_dirname(vault_name)).await?;
    save_vault(&dir_handle, imported_vault).await?;

    Ok(())
}

pub async fn get_vault(vault_name: &str) -> Result<(FileSystemDirectoryHandle, Vault), JsValue> {
    let dirname = get_vault_dirname(vault_name);
    read_vault_with_name(&dirname)
        .await
        .map_err(|_| JsValue::from_str(&format!("Vault '{}' does not exist", vault_name)))
}

pub async fn read_vault_with_name(
    vault_name: &str,
) -> Result<(FileSystemDirectoryHandle, Vault), VaultError> {
    let root = get_root_directory_handle().await?;
    let dirname = get_vault_dirname(vault_name);

    let dir_handle = JsFuture::from(root.get_directory_handle(&dirname))
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to get directory handle",
        })?
        .unchecked_into::<FileSystemDirectoryHandle>();

    let metadata_handle =
        get_or_create_file_handle_in_directory(&dir_handle, &get_metadata_filename()).await?;

    let file = JsFuture::from(metadata_handle.get_file())
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to get metadata file",
        })?;

    let metadata_text = JsFuture::from(file.unchecked_into::<web_sys::File>().text())
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to read metadata file",
        })?
        .as_string()
        .ok_or(VaultError::IoError {
            message: "Failed to convert metadata data to string",
        })?;

    let mut vault: Vault =
        serde_json::from_str(&metadata_text).map_err(|_| VaultError::SerializationError {
            message: "Failed to deserialize vault metadata",
        })?;

    vault.namespaces.clear();

    if let Ok(entries_val) = js_sys::Reflect::get(&dir_handle, &JsValue::from_str("entries")) {
        if let Some(entries_fn) = entries_val.dyn_ref::<js_sys::Function>() {
            if let Ok(iterator) = entries_fn.call0(&dir_handle) {
                loop {
                    let next_val = js_sys::Reflect::get(&iterator, &JsValue::from_str("next"))
                        .map_err(|_| VaultError::IoError {
                            message: "Failed to get next entry",
                        })?;

                    if let Some(next_fn) = next_val.dyn_ref::<js_sys::Function>() {
                        if let Ok(promise) = next_fn.call0(&iterator) {
                            let next_result = JsFuture::from(
                                promise.dyn_into::<js_sys::Promise>().map_err(|_| {
                                    VaultError::IoError {
                                        message: "Failed to convert next result to promise",
                                    }
                                })?,
                            )
                            .await
                            .map_err(|_| VaultError::IoError {
                                message: "Failed to get next entry",
                            })?;

                            let done =
                                js_sys::Reflect::get(&next_result, &JsValue::from_str("done"))
                                    .map_err(|_| VaultError::IoError {
                                        message: "Failed to get done status",
                                    })?
                                    .as_bool()
                                    .unwrap_or(true);

                            if done {
                                break;
                            }

                            if next_result.is_null() {
                                break;
                            }

                            if let Ok(value) =
                                js_sys::Reflect::get(&next_result, &JsValue::from_str("value"))
                            {
                                if let Some(array) = value.dyn_ref::<js_sys::Array>() {
                                    if let Some(name) = array.get(0).as_string() {
                                        // Only process .ns files
                                        if name.ends_with(".ns") {
                                            let handle = array
                                                .get(1)
                                                .dyn_into::<web_sys::FileSystemFileHandle>()
                                                .map_err(|_| VaultError::IoError {
                                                    message: "Failed to get file handle",
                                                })?;

                                            let file = JsFuture::from(handle.get_file())
                                                .await
                                                .map_err(|_| VaultError::IoError {
                                                    message: "Failed to get namespace file",
                                                })?;

                                            let namespace_text = JsFuture::from(
                                                file.unchecked_into::<web_sys::File>().text(),
                                            )
                                            .await
                                            .map_err(|_| VaultError::IoError {
                                                message: "Failed to read namespace file",
                                            })?
                                            .as_string()
                                            .ok_or(VaultError::IoError {
                                                message:
                                                    "Failed to convert namespace data to string",
                                            })?;

                                            let namespace_data: NamespaceData =
                                                serde_json::from_str(&namespace_text).map_err(
                                                    |_| VaultError::SerializationError {
                                                        message:
                                                            "Failed to deserialize namespace data",
                                                    },
                                                )?;

                                            let namespace = name[..name.len() - 3].to_string();
                                            vault.namespaces.insert(namespace, namespace_data);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok((dir_handle, vault))
}

pub async fn save_vault(
    dir_handle: &FileSystemDirectoryHandle,
    vault: Vault,
) -> Result<(), VaultError> {
    if !has_requested_persistence() {
        let is_persisted = check_storage_persistence().await.unwrap_or(false);

        if !is_persisted {
            let result = request_persistence_storage().await;

            match result {
                Ok(is_granted) => {
                    logger().log(&format!("persistence request granted: {}", is_granted));
                }
                Err(VaultError::JsError(message)) => {
                    logger().error(&message);
                }
                _ => {}
            }
        }
    }

    let mut namespace_data = Vec::new();
    for (namespace, data) in &vault.namespaces {
        let namespace_json = serde_json::to_string(&data).map_err(|_| VaultError::IoError {
            message: "Failed to serialize namespace data",
        })?;
        namespace_data.push((namespace.clone(), namespace_json));
    }

    let mut metadata_vault = vault.clone();
    metadata_vault.namespaces.clear();

    let metadata_handle =
        get_or_create_file_handle_in_directory(dir_handle, &get_metadata_filename())
            .await
            .map_err(VaultError::from)?;

    let metadata_json =
        serde_json::to_string(&metadata_vault).map_err(|_| VaultError::IoError {
            message: "Failed to serialize vault metadata",
        })?;

    let writer = JsFuture::from(metadata_handle.create_writable())
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to create writable",
        })?;

    let promise = writer
        .unchecked_ref::<web_sys::FileSystemWritableFileStream>()
        .write_with_str(&metadata_json)
        .map_err(|_| VaultError::IoError {
            message: "Failed to create Promise for writing vault metadata",
        })?;

    JsFuture::from(promise)
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to write vault metadata",
        })?;

    JsFuture::from(
        writer
            .unchecked_ref::<web_sys::FileSystemWritableFileStream>()
            .close(),
    )
    .await
    .map_err(|_| VaultError::IoError {
        message: "Failed to close writer",
    })?;

    for (namespace, namespace_json) in namespace_data {
        let file_handle =
            get_or_create_file_handle_in_directory(dir_handle, &get_namespace_filename(&namespace))
                .await?;

        let writer = JsFuture::from(file_handle.create_writable())
            .await
            .map_err(|_| VaultError::IoError {
                message: "Failed to create writable",
            })?;

        match writer
            .unchecked_ref::<web_sys::FileSystemWritableFileStream>()
            .write_with_str(&namespace_json)
        {
            Ok(promise) => {
                JsFuture::from(promise)
                    .await
                    .map_err(|_| VaultError::IoError {
                        message: "Failed to write namespace data",
                    })?;
            }
            Err(_) => {
                return Err(VaultError::IoError {
                    message: "Failed to create Promise for writing namespace data",
                });
            }
        }

        JsFuture::from(
            writer
                .unchecked_ref::<web_sys::FileSystemWritableFileStream>()
                .close(),
        )
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to close writer",
        })?;
    }

    let global_scope = get_global_scope()?;
    let msg = notifications::Message {
        event: notifications::EventType::VaultUpdate,
        data: vault.clone(),
    };
    let js_value = serde_wasm_bindgen::to_value(&msg).map_err(|_| VaultError::IoError {
        message: "Failed to serialize",
    })?;

    if let Ok(worker_scope) = global_scope
        .clone()
        .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
    {
        worker_scope.post_message(&js_value)?;
        logger().log("message posted using worker");
    } else if let Ok(window) = global_scope.dyn_into::<web_sys::Window>() {
        window
            .post_message(&js_value, "*")
            .map_err(|_| VaultError::IoError {
                message: "Failed to post message to window",
            })?;
        logger().log("message posted using window");
    } else {
        return Err(VaultError::IoError {
            message: "Unknown global scope",
        });
    }

    Ok(())
}

async fn cleanup_expired_data(
    vault: &mut Vault,
    dir_handle: &FileSystemDirectoryHandle,
) -> Result<bool, VaultError> {
    let now = js_sys::Date::now() as i64 / 1000;
    let mut data_removed = false;

    let expired_namespaces: Vec<String> = vault
        .namespaces
        .iter()
        .filter_map(|(namespace, encrypted)| {
            if let Some(expiration) = &encrypted.expiration {
                if now >= expiration.expires_at {
                    Some(namespace.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    for namespace in expired_namespaces {
        let filename = get_namespace_filename(&namespace);
        let _ = remove_file_from_directory(dir_handle, &filename).await;
        vault.namespaces.remove(&namespace);
        data_removed = true;
        logger().log(&format!("Removed expired namespace: {}", namespace));
    }

    if data_removed {
        save_vault(dir_handle, vault.clone()).await?;
    }

    Ok(data_removed)
}

#[wasm_bindgen]
pub async fn force_cleanup_vault(vault_name: &str) -> Result<(), JsValue> {
    let _lock = acquire_vault_lock(vault_name).await?;
    let (file_handle, mut vault) = read_vault_with_name(vault_name).await?;

    while cleanup_expired_data(&mut vault, &file_handle).await? {}

    Ok(())
}

async fn insert_namespace_data(
    identity: &IdentityHandle,
    data: JsValue,
    expires_in_seconds: Option<i64>,
) -> Result<NamespaceData, JsValue> {
    let data_json = from_value::<serde_json::Value>(data)?;
    let data_bytes =
        serde_json::to_vec(&data_json).map_err(|_| VaultError::SerializationError {
            message: "Failed to serialize data",
        })?;

    let recipient = identity.to_public();
    let encrypted_data = encrypt_with_recipients(&data_bytes, &[recipient]).await?;

    let expiration = if let Some(seconds) = expires_in_seconds {
        if seconds <= 0 {
            None
        } else {
            let now = js_sys::Date::now() as i64 / 1000;
            Some(Expiration {
                expires_at: now + seconds,
            })
        }
    } else {
        None
    };

    let namespace_data = NamespaceData {
        data: encrypted_data,
        expiration,
    };

    Ok(namespace_data)
}

static CLEANUP_INTERVAL: AtomicI64 = AtomicI64::new(0);
static LAST_CLEANUP: AtomicI64 = AtomicI64::new(0);

// TODO: enable sync by namespace
#[wasm_bindgen]
pub async fn enable_sync(
    vault_name: &str,
    identity: &IdentityHandle,
    signaling_url: JsValue,
    stun_servers: JsValue,
) -> Result<JsValue, JsValue> {
    let signaling_url_str: String = from_value(signaling_url)?;
    let stun_servers_vec: Vec<String> = from_value(stun_servers)?;

    let mut vault = check_identity(vault_name, identity).await?;

    if vault.metadata.peer_id.is_none() {
        let mut rng = OsRng;
        let mut peer_id = [0u8; 16];
        rng.fill_bytes(&mut peer_id);
        vault.metadata.peer_id = Some(hex::encode(peer_id));
    }

    let mut updated_vault = vault.clone();
    updated_vault.sync_enabled = true;

    let dir_handle = get_or_create_directory_handle(&get_vault_dirname(vault_name)).await?;
    let vault = updated_vault.clone();

    save_vault(&dir_handle, vault.clone()).await?;

    let (mut peer, _receiver): (WebRtcPeer, UnboundedReceiver<Vec<u8>>) =
        WebRtcPeer::create_peer(vault.metadata.peer_id.clone().unwrap(), stun_servers_vec).await?;

    logger().log(&format!(
        "Connecting to signaling server at {}",
        signaling_url_str
    ));

    logger().log("Connecting to signaling server...");
    peer.connect(&signaling_url_str, None).await?;

    Ok(JsValue::from_str(&vault.metadata.peer_id.clone().unwrap()))
}

#[wasm_bindgen]
pub async fn connect_to_peer(
    vault_name: &str,
    identity: &IdentityHandle,
    peer_id: JsValue,
    signaling_url: JsValue,
) -> Result<(), JsValue> {
    logger().log(&format!(
        "connect_to_peer called with: vault_name = {}",
        vault_name
    ));
    logger().log(&format!("identity = {:?}", identity));
    logger().log(&format!("peer_id = {:?}", peer_id));
    logger().log(&format!("signaling_url = {:?}", signaling_url));

    let peer_id_str: String = from_value(peer_id)?;
    let signaling_url_str: String = from_value(signaling_url)?;

    logger().log("Checking identity...");
    let vault = check_identity(vault_name, identity).await?;

    if !vault.sync_enabled {
        let msg = "Sync is not enabled for this vault";
        logger().error(msg);
        return Err(JsValue::from_str(msg));
    }

    let my_peer_id = vault
        .metadata
        .peer_id
        .clone()
        .ok_or_else(|| JsValue::from_str("No peer ID found in vault metadata"))?;

    logger().log("Creating WebRTC peer...");
    let stun_servers = js_sys::Array::new();
    stun_servers.push(&"stun:stun.l.google.com:19302".into());
    let stun_servers: Vec<String> = stun_servers
        .iter()
        .map(|s| s.as_string().unwrap_or_default())
        .collect();

    let (peer, _receiver): (WebRtcPeer, UnboundedReceiver<Vec<u8>>) =
        WebRtcPeer::create_peer(my_peer_id, stun_servers).await?;
    let peer_rc = Rc::new(RefCell::new(peer));

    logger().log("Connecting to signaling server...");
    peer_rc
        .borrow_mut()
        .connect(&signaling_url_str, Some(&peer_id_str))
        .await?;

    logger().log("Adding peer to sync manager...");
    logger().log(&format!("Adding peer {} to sync manager", peer_id_str));
    let sync_manager = get_sync_manager(vault_name)?;
    sync_manager.borrow_mut().add_peer(peer_rc);

    logger().log("Sending initial vault data to the peer...");
    let vault = read_vault_with_name(vault_name).await?.1;
    let sync_manager = get_sync_manager(vault_name)?;
    let mut sync_manager = sync_manager.borrow_mut();

    let mut operations = Vec::new();
    logger().log(&format!(
        "Found {} namespaces to sync",
        vault.namespaces.len()
    ));

    for (namespace, data) in vault.namespaces {
        logger().log(&format!("Creating operation for namespace: {}", namespace));
        let operation =
            sync_manager.create_operation(namespace, OperationType::Insert, Some(data.data), None);
        operations.push(operation);
    }

    let peer = if let Some(p) = sync_manager.get_peers_mut().get(&peer_id_str) {
        p.clone()
    } else {
        return Err(JsValue::from_str("Peer not found in sync manager"));
    };

    let sync_messages: Vec<_> = operations
        .into_iter()
        .map(|op| {
            sync_manager.create_sync_message(
                vault_name.to_string(),
                op,
                Some(vault.metadata.clone()),
                Some(vault.identity_salts.clone()),
                Some(vault.username_pk.clone()),
            )
        })
        .collect();

    let peer_ref = peer.borrow();

    let mut retries = 0;
    while !peer_ref.is_ready() && retries < 20 {
        logger().log(&format!(
            "Waiting for WebRTC connection to be ready (attempt {})",
            retries + 1
        ));
        logger().log(&format!(
            "Connection status: connected={}, channel_open={}, ice_connected={}",
            peer_ref.is_connected(),
            peer_ref.is_channel_open(),
            peer_ref.is_ice_connected()
        ));
        gloo_timers::future::TimeoutFuture::new(1000).await;
        retries += 1;
    }

    if !peer_ref.is_ready() {
        return Err(JsValue::from_str(&format!(
            "WebRTC connection not ready after timeout. Status: connected={}, channel_open={}, ice_connected={}. Make sure both peers are connected and the data channel is open.",
            peer_ref.is_connected(),
            peer_ref.is_channel_open(),
            peer_ref.is_ice_connected()
        )));
    }

    for sync_msg in sync_messages {
        logger().log(&format!(
            "Sending sync message for vault: {}, namespace: {}",
            vault_name, sync_msg.operation.namespace
        ));

        let msg_bytes = serde_json::to_vec(&sync_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize sync message: {}", e)))?;

        peer_ref.send_message(msg_bytes)?;
        logger().log("Sync message sent successfully");
    }
    Ok(())
}

#[wasm_bindgen]
pub async fn add_peer(
    vault_name: &str,
    identity: &IdentityHandle,
    peer_id: JsValue,
    namespace: JsValue,
    access_level: JsValue,
) -> Result<(), JsValue> {
    let peer_id_str: String = from_value(peer_id)?;
    let namespace_str: String = from_value(namespace)?;
    let access_level_str: String = from_value(access_level)?;

    let vault = check_identity(vault_name, identity).await?;

    if !vault.sync_enabled {
        return Err(JsValue::from_str("Sync is not enabled for this vault"));
    }

    let access_level = match access_level_str.as_str() {
        "viewer" => AccessLevel::Viewer,
        "contributor" => AccessLevel::Contributor,
        "administrator" => AccessLevel::Administrator,
        _ => return Err(JsValue::from_str("Invalid access level")),
    };

    let sync_manager = get_sync_manager(vault_name)?;
    let mut sync_manager = sync_manager.borrow_mut();

    if let Some(peer) = sync_manager.get_peers_mut().get(&peer_id_str) {
        peer.borrow_mut()
            .add_permission(namespace_str.clone(), access_level);
    } else {
        return Err(JsValue::from_str("Peer not found in sync manager"));
    }

    if let Some(namespace_data) = vault.namespaces.get(&namespace_str) {
        logger().log(&format!(
            "Found data for namespace {}, preparing to send",
            namespace_str
        ));

        let peer = if let Some(p) = sync_manager.get_peers_mut().get(&peer_id_str) {
            p.clone()
        } else {
            return Err(JsValue::from_str("Peer not found in sync manager"));
        };

        let operation = sync_manager.create_operation(
            namespace_str.clone(),
            OperationType::Insert,
            Some(namespace_data.data.clone()),
            None,
        );

        let sync_msg = sync_manager.create_sync_message(
            vault_name.to_string(),
            operation,
            Some(vault.metadata.clone()),
            Some(vault.identity_salts.clone()),
            Some(vault.username_pk.clone()),
        );

        let peer_ref = peer.borrow();

        let mut retries = 0;
        while !peer_ref.is_ready() && retries < 10 {
            logger().log(&format!(
                "Waiting for WebRTC connection to be ready (attempt {})",
                retries + 1
            ));
            gloo_timers::future::TimeoutFuture::new(500).await;
            retries += 1;
        }

        if !peer_ref.is_ready() {
            return Err(JsValue::from_str(
                "WebRTC connection not ready after timeout",
            ));
        }

        logger().log(&format!(
            "Sending data for namespace {} to peer",
            namespace_str
        ));

        let msg_bytes = serde_json::to_vec(&sync_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize sync message: {}", e)))?;

        peer_ref.send_message(msg_bytes)?;
        logger().log("Data sent successfully");
    } else {
        logger().log(&format!("No data found for namespace {}", namespace_str));
    }

    Ok(())
}

#[wasm_bindgen]
pub async fn update_vault_from_sync(vault_name: &str, vault_data: &[u8]) -> Result<(), VaultError> {
    let sync_msg: SyncMessage = serde_json::from_slice(vault_data)
        .map_err(|e| VaultError::JsError(format!("Failed to deserialize sync message: {:?}", e)))?;

    let (file_handle, mut current_vault) = match read_vault_with_name(vault_name).await {
        Ok((handle, vault)) => (handle, vault),
        Err(VaultError::IoError {
            message: "Failed to get directory handle",
        }) => {
            logger().log(&format!("Creating new vault {} for sync", vault_name));

            let dirname = get_vault_dirname(vault_name);
            let dir_handle = get_or_create_directory_handle(&dirname).await?;

            let vault = Vault {
                metadata: sync_msg.vault_metadata.ok_or_else(|| {
                    VaultError::JsError(
                        "Missing vault metadata in sync message for new vault".to_string(),
                    )
                })?,
                identity_salts: sync_msg
                    .identity_salts
                    .clone()
                    .unwrap_or_else(IdentitySalts::new),
                username_pk: match sync_msg.username_pk {
                    Some(username_pk) => username_pk,
                    None => HashMap::new(),
                },
                namespaces: HashMap::new(),
                sync_enabled: true,
            };

            save_vault(&dir_handle, vault.clone()).await?;

            (dir_handle, vault)
        }
        Err(e) => return Err(e),
    };

    // Update identity salts if provided in sync message
    if let Some(salts) = sync_msg.identity_salts {
        current_vault.identity_salts = salts;
    }

    match sync_msg.operation.operation_type {
        OperationType::Insert | OperationType::Update => {
            if let (Some(data), _) = (sync_msg.operation.data, sync_msg.operation.nonce) {
                let namespace = sync_msg.operation.namespace.clone();
                let namespace_data = NamespaceData {
                    data,
                    expiration: None,
                };
                current_vault
                    .namespaces
                    .insert(namespace.clone(), namespace_data.clone());
                logger().log(&format!("Updated namespace {} in vault", namespace));
            }
        }
        OperationType::Delete => {
            let namespace = sync_msg.operation.namespace.clone();
            current_vault.namespaces.remove(&namespace);
            logger().log(&format!("Removed namespace {} from vault", namespace));
        }
    }

    save_vault(&file_handle, current_vault).await?;

    Ok(())
}

#[wasm_bindgen]
pub fn configure_cleanup(interval_seconds: i64) {
    if interval_seconds > 0 {
        logger().log(&format!(
            "Configuring cleanup with interval of {} seconds",
            interval_seconds
        ));
        CLEANUP_INTERVAL.store(interval_seconds, Ordering::SeqCst);
        LAST_CLEANUP.store(js_sys::Date::now() as i64 / 1000, Ordering::SeqCst);
    } else {
        logger().log("Disabling automatic cleanup");
        CLEANUP_INTERVAL.store(0, Ordering::SeqCst);
    }
}
