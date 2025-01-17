use crate::crypto::derive_key;
use crate::errors::VaultError;
use crate::file_system::{
    get_or_create_directory_handle, get_or_create_file_handle_in_directory,
    get_root_directory_handle, remove_directory_with_contents, remove_file_from_directory,
};
use crate::lock::acquire_vault_lock;
use crate::measure::get_performance;
use crate::measure::time_it;
use crate::sync::{get_sync_manager, OperationType, SyncMessage};
use crate::webrtc::{AccessLevel, WebRtcPeer};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::console;
use core::str;
use serde_wasm_bindgen::{from_value, to_value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use wasm_bindgen_futures::JsFuture;

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};

use web_sys::{self, FileSystemDirectoryHandle};

use argon2::password_hash::rand_core::OsRng;

use rand::RngCore;

use futures_channel::mpsc::UnboundedReceiver;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Expiration {
    expires_at: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct NamespaceData {
    data: Vec<u8>,
    nonce: [u8; 12],
    expiration: Option<Expiration>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct VaultMetadata {
    pub salt: [u8; 32],
    pub peer_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Vault {
    pub metadata: VaultMetadata,
    pub namespaces: HashMap<String, NamespaceData>,
    pub sync_enabled: bool,
}

fn get_vault_dirname(vault_name: &str) -> String {
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

#[wasm_bindgen]
pub async fn upsert_vault(
    vault_name: &str,
    password: JsValue,
    namespace: JsValue,
    data: JsValue,
    expires_in_seconds: Option<i64>,
    replace_if_exists: bool,
) -> Result<(), JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    let password_str: String = from_value(password.clone())?;

    validate_namespace(&namespace_str)?;

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

        let namespace_str = namespace_str.clone();
        let password_str = password_str.clone();
        let vault_name = vault_name.to_string();

        let result = async {
            let mut vault = match check_password(&vault_name, &password_str).await {
                Ok(vault) => vault,
                Err(VaultError::IoError { message })
                    if message.contains("Failed to get directory handle") =>
                {
                    create_vault(
                        JsValue::from_str(&vault_name),
                        password.clone(),
                        namespace.clone(),
                        data.clone(),
                        expires_in_seconds,
                    )
                    .await?;
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };

            if vault.namespaces.contains_key(&namespace_str) && !replace_if_exists {
                return Err(VaultError::NamespaceAlreadyExists.into());
            }

            let namespace_data = insert_namespace_data(
                &mut vault,
                password.clone(),
                data.clone(),
                expires_in_seconds,
            )
            .await?;
            vault
                .namespaces
                .insert(namespace_str.clone(), namespace_data.clone());

            let (dir_handle, _) = read_vault_with_name(&vault_name).await?;
            save_vault(&dir_handle, vault.clone()).await?;

            if let Ok(sync_manager) = get_sync_manager(&vault_name) {
                let mut sync_manager = sync_manager.borrow_mut();

                let operation = sync_manager.create_operation(
                    namespace_str,
                    OperationType::Update,
                    Some(namespace_data.data),
                    Some(namespace_data.nonce),
                );

                let sync_msg = sync_manager.create_sync_message(
                    vault_name.to_string(),
                    operation,
                    Some(vault.metadata.clone()),
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
    password: JsValue,
    namespace: JsValue,
) -> Result<(), JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    let password: String = from_value(password)?;
    let namespace: String = from_value(namespace)?;

    check_password(vault_name, &password).await?;

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
    password: JsValue,
    namespace: JsValue,
) -> Result<JsValue, JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    let namespace_str = namespace.as_string().unwrap_or_default();
    if get_performance().is_some() {
        console::time(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    let result = time_it!("Total read_from_vault", {
        let password: String = from_value(password).map_err(|_| VaultError::IoError {
            message: "Invalid password format",
        })?;
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

        let key_bytes = time_it!("Key derivation", {
            derive_key(password.as_bytes(), &vault.metadata.salt)?
        });

        let cipher_key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(cipher_key);
        let nonce = Nonce::from_slice(&encrypted_namespace.nonce);

        let decrypted_data = time_it!("Decryption", {
            cipher
                .decrypt(nonce, encrypted_namespace.data.as_ref())
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
        console::timeEnd(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    result
}

#[wasm_bindgen]
pub async fn list_namespaces(vault_name: &str, password: JsValue) -> Result<JsValue, JsValue> {
    let password: String = from_value(password)?;

    let (_, vault) = match read_vault_with_name(vault_name).await {
        Ok(result) => result,
        Err(VaultError::IoError { .. }) => {
            return Ok(to_value(&Vec::<String>::new())?);
        }
        Err(e) => return Err(e.into()),
    };

    console::log(&format!(
        "Found {} namespaces in vault",
        vault.namespaces.len()
    ));
    for key in vault.namespaces.keys() {
        console::log(&format!("Namespace found: {}", key));
    }

    check_password(vault_name, &password).await?;

    let namespaces: Vec<String> = vault.namespaces.keys().cloned().collect();

    console::log(&format!("Returning {} namespaces", namespaces.len()));
    Ok(to_value(&namespaces)?)
}

#[wasm_bindgen]
pub async fn remove_vault(vault_name: &str, password: JsValue) -> Result<(), JsValue> {
    let _lock = acquire_vault_lock(vault_name).await?;
    let password: String = from_value(password)?;
    check_password(vault_name, &password).await?;

    let dirname = get_vault_dirname(vault_name);
    let root_handle = get_root_directory_handle().await?;
    remove_directory_with_contents(&root_handle, &dirname).await?;

    Ok(())
}

#[wasm_bindgen]
pub async fn list_vaults() -> Result<JsValue, JsValue> {
    let root = get_root_directory_handle().await?;
    console::log("Listing vaults from root directory");

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

        if let Ok(value) = js_sys::Reflect::get(&next_result, &JsValue::from_str("value"))?
            .dyn_into::<js_sys::Array>()
        {
            if let Some(name) = value.get(0).as_string() {
                let vault_name = name;
                console::log(&format!("Found vault: {}", vault_name));
                vault_names.push(vault_name);
            }
        }
    }

    console::log(&format!("Found {} vaults in total", vault_names.len()));
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
pub async fn create_vault(
    vault_name: JsValue,
    password: JsValue,
    namespace: JsValue,
    data: JsValue,
    expires_in_seconds: Option<i64>,
) -> Result<(), JsValue> {
    let vault_name_str: String = from_value(vault_name)?;
    let namespace_str: String = from_value(namespace.clone())?;
    let dirname = get_vault_dirname(&vault_name_str);

    validate_vault_name(&vault_name_str)?;
    validate_namespace(&namespace_str)?;

    let _lock = acquire_vault_lock(&vault_name_str).await?;

    if (read_vault_with_name(&vault_name_str).await).is_ok() {
        return Err(VaultError::VaultAlreadyExists.into());
    }

    let dir_handle = get_or_create_directory_handle(&dirname).await?;

    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);

    let metadata = VaultMetadata {
        salt,
        peer_id: None,
    };

    let mut vault = Vault {
        metadata,
        namespaces: HashMap::new(),
        sync_enabled: false,
    };

    let namespace_data =
        insert_namespace_data(&mut vault, password, data, expires_in_seconds).await?;
    vault.namespaces.insert(namespace_str, namespace_data);
    save_vault(&dir_handle, vault).await?;

    console::log(&format!("Vault created successfully: {}", dirname));
    Ok(())
}

async fn check_password(vault_name: &str, password: &str) -> Result<Vault, VaultError> {
    let (_, vault) = match read_vault_with_name(vault_name).await {
        Ok((handle, existing_vault)) => (handle, existing_vault),

        Err(VaultError::IoError { .. }) => {
            return Err(VaultError::VaultNotFound);
        }

        Err(e) => return Err(e),
    };

    if let Some((_, first_encrypted)) = vault.namespaces.iter().next() {
        let key_bytes = derive_key(password.as_bytes(), &vault.metadata.salt)?;
        let cipher_key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(cipher_key);

        cipher
            .decrypt(
                Nonce::from_slice(&first_encrypted.nonce),
                first_encrypted.data.as_ref(),
            )
            .map_err(|_| VaultError::InvalidPassword)?;
    }
    Ok(vault)
}

#[wasm_bindgen]
pub async fn export_vault(vault_name: &str, password: JsValue) -> Result<JsValue, JsValue> {
    let password: String = from_value(password)?;

    let vault = check_password(vault_name, &password).await?;

    // Create binary format with magic number "VAULT1"
    let magic = b"VAULT1";
    let serialized = serde_json::to_vec(&vault).map_err(|e| {
        console::log(&format!("Serialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to serialize vault for export",
        }
    })?;

    let total_size = magic.len() + 4 + serialized.len();
    let mut vault_bytes = Vec::with_capacity(total_size);

    vault_bytes.extend_from_slice(magic);
    vault_bytes.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
    vault_bytes.extend_from_slice(&serialized);

    console::log(&format!(
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

    console::log(&format!(
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
        console::log(&format!("Deserialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to deserialize vault data",
        }
    })?;

    match read_vault_with_name(vault_name).await {
        Ok(_) => {
            return Err(VaultError::VaultAlreadyExists.into());
        }
        Err(VaultError::IoError { .. }) => {
            console::log(&format!(
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

async fn read_vault_with_name(
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

                            let value =
                                js_sys::Reflect::get(&next_result, &JsValue::from_str("value"))
                                    .map_err(|_| VaultError::IoError {
                                        message: "Failed to get entry value",
                                    })?;

                            let array = value.dyn_into::<js_sys::Array>().map_err(|_| {
                                VaultError::IoError {
                                    message: "Failed to convert entry to array",
                                }
                            })?;

                            let name = array.get(0).as_string().ok_or(VaultError::IoError {
                                message: "Failed to get entry name",
                            })?;

                            // Only process .ns files
                            if name.ends_with(".ns") {
                                let handle = array
                                    .get(1)
                                    .dyn_into::<web_sys::FileSystemFileHandle>()
                                    .map_err(|_| VaultError::IoError {
                                        message: "Failed to get file handle",
                                    })?;

                                let file =
                                    JsFuture::from(handle.get_file()).await.map_err(|_| {
                                        VaultError::IoError {
                                            message: "Failed to get namespace file",
                                        }
                                    })?;

                                let namespace_text =
                                    JsFuture::from(file.unchecked_into::<web_sys::File>().text())
                                        .await
                                        .map_err(|_| VaultError::IoError {
                                            message: "Failed to read namespace file",
                                        })?
                                        .as_string()
                                        .ok_or(VaultError::IoError {
                                            message: "Failed to convert namespace data to string",
                                        })?;

                                let namespace_data: NamespaceData =
                                    serde_json::from_str(&namespace_text).map_err(|_| {
                                        VaultError::SerializationError {
                                            message: "Failed to deserialize namespace data",
                                        }
                                    })?;

                                let namespace = name[..name.len() - 3].to_string();
                                vault.namespaces.insert(namespace, namespace_data);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok((dir_handle, vault))
}

async fn save_vault(
    dir_handle: &FileSystemDirectoryHandle,
    vault: Vault,
) -> Result<(), VaultError> {
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
        console::log(&format!("Removed expired namespace: {}", namespace));
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
    vault: &mut Vault,
    password: JsValue,
    data: JsValue,
    expires_in_seconds: Option<i64>,
) -> Result<NamespaceData, JsValue> {
    let password: String = from_value(password)?;

    let data_json = from_value::<serde_json::Value>(data)?;
    let data_bytes =
        serde_json::to_vec(&data_json).map_err(|_| VaultError::SerializationError {
            message: "Failed to serialize data",
        })?;

    let key_bytes = derive_key(password.as_bytes(), &vault.metadata.salt)?;
    let cipher_key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(cipher_key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted_data =
        cipher
            .encrypt(nonce, data_bytes.as_ref())
            .map_err(|_| VaultError::IoError {
                message: "Encryption failed",
            })?;

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
        nonce: nonce_bytes,
        expiration,
    };

    Ok(namespace_data)
}

static CLEANUP_INTERVAL: AtomicI64 = AtomicI64::new(0);
static LAST_CLEANUP: AtomicI64 = AtomicI64::new(0);

#[wasm_bindgen]
pub async fn enable_sync(
    vault_name: &str,
    password: JsValue,
    signaling_url: JsValue,
    stun_servers: JsValue,
) -> Result<JsValue, JsValue> {
    let password_str: String = from_value(password)?;
    let signaling_url_str: String = from_value(signaling_url)?;
    let stun_servers_vec: Vec<String> = from_value(stun_servers)?;

    let mut vault = check_password(vault_name, &password_str).await?;

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

    console::log(&format!(
        "Connecting to signaling server at {}",
        signaling_url_str
    ));

    console::log("Connecting to signaling server...");
    peer.connect(&signaling_url_str, None).await?;

    Ok(JsValue::from_str(&vault.metadata.peer_id.clone().unwrap()))
}

#[wasm_bindgen]
pub async fn connect_to_peer(
    vault_name: &str,
    password: JsValue,
    peer_id: JsValue,
    signaling_url: JsValue,
) -> Result<(), JsValue> {
    console::log(&format!(
        "connect_to_peer called with: vault_name = {}",
        vault_name
    ));
    console::log(&format!("password = {:?}", password));
    console::log(&format!("peer_id = {:?}", peer_id));
    console::log(&format!("signaling_url = {:?}", signaling_url));

    let password_str: String = from_value(password)?;
    let peer_id_str: String = from_value(peer_id)?;
    let signaling_url_str: String = from_value(signaling_url)?;

    console::log("Checking password...");
    let vault = check_password(vault_name, &password_str).await?;

    if !vault.sync_enabled {
        let msg = "Sync is not enabled for this vault";
        console::error(msg);
        return Err(JsValue::from_str(msg));
    }

    let my_peer_id = vault
        .metadata
        .peer_id
        .clone()
        .ok_or_else(|| JsValue::from_str("No peer ID found in vault metadata"))?;

    console::log("Creating WebRTC peer...");
    let stun_servers = js_sys::Array::new();
    stun_servers.push(&"stun:stun.l.google.com:19302".into());
    let stun_servers: Vec<String> = stun_servers
        .iter()
        .map(|s| s.as_string().unwrap_or_default())
        .collect();

    let (peer, _receiver): (WebRtcPeer, UnboundedReceiver<Vec<u8>>) =
        WebRtcPeer::create_peer(my_peer_id, stun_servers).await?;
    let peer_rc = Rc::new(RefCell::new(peer));

    console::log("Connecting to signaling server...");
    peer_rc
        .borrow_mut()
        .connect(&signaling_url_str, Some(&peer_id_str))
        .await?;

    console::log("Adding peer to sync manager...");
    console::log(&format!("Adding peer {} to sync manager", peer_id_str));
    let sync_manager = get_sync_manager(vault_name)?;
    sync_manager.borrow_mut().add_peer(peer_rc);

    console::log("Sending initial vault data to the peer...");
    let vault = read_vault_with_name(vault_name).await?.1;
    let sync_manager = get_sync_manager(vault_name)?;
    let mut sync_manager = sync_manager.borrow_mut();

    let mut operations = Vec::new();
    console::log(&format!(
        "Found {} namespaces to sync",
        vault.namespaces.len()
    ));

    for (namespace, data) in vault.namespaces {
        console::log(&format!("Creating operation for namespace: {}", namespace));
        let operation = sync_manager.create_operation(
            namespace,
            OperationType::Insert,
            Some(data.data),
            Some(data.nonce),
        );
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
            )
        })
        .collect();

    let peer_ref = peer.borrow();

    let mut retries = 0;
    while !peer_ref.is_ready() && retries < 20 {
        console::log(&format!(
            "Waiting for WebRTC connection to be ready (attempt {})",
            retries + 1
        ));
        console::log(&format!(
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
        console::log(&format!(
            "Sending sync message for vault: {}, namespace: {}",
            vault_name, sync_msg.operation.namespace
        ));

        let msg_bytes = serde_json::to_vec(&sync_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize sync message: {}", e)))?;

        peer_ref.send_message(msg_bytes)?;
        console::log("Sync message sent successfully");
    }
    Ok(())
}

#[wasm_bindgen]
pub async fn add_peer(
    vault_name: &str,
    password: JsValue,
    peer_id: JsValue,
    namespace: JsValue,
    access_level: JsValue,
) -> Result<(), JsValue> {
    let password_str: String = from_value(password)?;
    let peer_id_str: String = from_value(peer_id)?;
    let namespace_str: String = from_value(namespace)?;
    let access_level_str: String = from_value(access_level)?;

    let vault = check_password(vault_name, &password_str).await?;

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
        console::log(&format!(
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
            Some(namespace_data.nonce),
        );

        let sync_msg = sync_manager.create_sync_message(
            vault_name.to_string(),
            operation,
            Some(vault.metadata.clone()),
        );

        let peer_ref = peer.borrow();

        let mut retries = 0;
        while !peer_ref.is_ready() && retries < 10 {
            console::log(&format!(
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

        console::log(&format!(
            "Sending data for namespace {} to peer",
            namespace_str
        ));

        let msg_bytes = serde_json::to_vec(&sync_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize sync message: {}", e)))?;

        peer_ref.send_message(msg_bytes)?;
        console::log("Data sent successfully");
    } else {
        console::log(&format!("No data found for namespace {}", namespace_str));
    }

    Ok(())
}

#[wasm_bindgen]
pub async fn update_vault_from_sync(vault_name: &str, vault_data: &[u8]) -> Result<(), VaultError> {
    let sync_msg: SyncMessage = serde_json::from_slice(vault_data)
        .map_err(|e| VaultError::JsError(format!("Failed to deserialize sync message: {}", e)))?;

    let (file_handle, mut current_vault) = match read_vault_with_name(vault_name).await {
        Ok((handle, vault)) => (handle, vault),
        Err(VaultError::IoError { message: "Failed to get directory handle" }) => {
            console::log(&format!("Creating new vault {} for sync", vault_name));

            let dirname = get_vault_dirname(vault_name);
            let dir_handle = get_or_create_directory_handle(&dirname).await?;

            let vault = Vault {
                metadata: sync_msg.vault_metadata.ok_or_else(|| {
                    VaultError::JsError(
                        "Missing vault metadata in sync message for new vault".to_string(),
                    )
                })?,
                namespaces: HashMap::new(),
                sync_enabled: true,
            };

            save_vault(&dir_handle, vault.clone()).await?;

            (dir_handle, vault)
        }
        Err(e) => return Err(e),
    };

    match sync_msg.operation.operation_type {
        OperationType::Insert | OperationType::Update => {
            if let (Some(data), Some(nonce)) = (sync_msg.operation.data, sync_msg.operation.nonce) {
                let namespace = sync_msg.operation.namespace.clone();
                let namespace_data = NamespaceData {
                    data,
                    nonce,
                    expiration: None,
                };
                current_vault
                    .namespaces
                    .insert(namespace.clone(), namespace_data);
                console::log(&format!("Updated namespace {} in vault", namespace));
            }
        }
        OperationType::Delete => {
            let namespace = sync_msg.operation.namespace.clone();
            current_vault.namespaces.remove(&namespace);
            console::log(&format!("Removed namespace {} from vault", namespace));
        }
    }

    save_vault(&file_handle, current_vault).await?;

    Ok(())
}

#[wasm_bindgen]
pub fn configure_cleanup(interval_seconds: i64) {
    if interval_seconds > 0 {
        console::log(&format!(
            "Configuring cleanup with interval of {} seconds",
            interval_seconds
        ));
        CLEANUP_INTERVAL.store(interval_seconds, Ordering::SeqCst);
        LAST_CLEANUP.store(js_sys::Date::now() as i64 / 1000, Ordering::SeqCst);
    } else {
        console::log("Disabling automatic cleanup");
        CLEANUP_INTERVAL.store(0, Ordering::SeqCst);
    }
}
