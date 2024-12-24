use crate::console::*;
use crate::crypto::derive_key;
use crate::errors::VaultError;
use crate::file_system::{get_or_create_file_handle_with_name, get_root_directory_handle};
use crate::lock::acquire_vault_lock;
use crate::measure::get_performance;
use crate::measure::time_it;
use wasm_bindgen::prelude::*;

use core::str;
use serde_wasm_bindgen::{from_value, to_value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use wasm_bindgen_futures::JsFuture;

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};

use web_sys::{self, FileSystemFileHandle, FileSystemGetFileOptions};

use argon2::password_hash::rand_core::OsRng;

use rand::RngCore;

use std::future::Future;
use std::pin::Pin;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Expiration {
    expires_at: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct EncryptedNamespace {
    data: Vec<u8>,
    nonce: [u8; 12],
    expiration: Option<Expiration>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Vault {
    namespaces: HashMap<String, EncryptedNamespace>,
    salt: [u8; 32],
}

fn validate_namespace(namespace: &str) -> Result<(), VaultError> {
    if namespace.trim().is_empty() {
        return Err(VaultError::IoError {
            message: "Namespace cannot be empty or whitespace only",
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

    check_password(vault_name, &password_str).await?;
    validate_namespace(&namespace_str)?;

    let (file_handle, mut vault) = match read_vault_with_name(vault_name).await {
        Ok((handle, existing_vault)) => {
            if existing_vault.namespaces.contains_key(&namespace_str) && !replace_if_exists {
                return Err(VaultError::NamespaceAlreadyExists.into());
            }
            (handle, existing_vault)
        }

        Err(VaultError::IoError { .. }) => {
            return Err(VaultError::VaultNotFound.into());
        }

        Err(e) => return Err(e.into()),
    };

    insert_namespace_data(&mut vault, password, namespace, data, expires_in_seconds).await?;

    save_vault(&file_handle, vault).await?;

    Ok(())
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

    let (file_handle, mut vault) = read_vault_with_name(vault_name).await?;

    if vault.namespaces.remove(&namespace).is_none() {
        return Err(VaultError::NamespaceNotFound.into());
    }

    save_vault(&file_handle, vault).await?;
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
        time(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    let result = time_it!("Total read_from_vault", {
        let password: String = from_value(password).map_err(|_| VaultError::IoError {
            message: "Invalid password format",
        })?;
        let namespace: String = from_value(namespace).map_err(|_| VaultError::IoError {
            message: "Invalid namespace format",
        })?;

        let (file_handle, mut vault) = match read_vault_with_name(vault_name).await {
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
                save_vault(&file_handle, vault.clone()).await?;
                return Err(VaultError::DataExpired.into());
            }
        }

        let key_bytes = time_it!("Key derivation", {
            derive_key(password.as_bytes(), &vault.salt)?
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
        timeEnd(&format!("read_from_vault {} {}", vault_name, namespace_str));
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

    log(&format!(
        "Found {} namespaces in vault",
        vault.namespaces.len()
    ));
    for key in vault.namespaces.keys() {
        log(&format!("Namespace found: {}", key));
    }

    check_password(vault_name, &password).await?;

    let namespaces: Vec<String> = vault.namespaces.keys().cloned().collect();

    log(&format!("Returning {} namespaces", namespaces.len()));
    Ok(to_value(&namespaces)?)
}

#[wasm_bindgen]
pub async fn remove_vault(vault_name: &str, password: JsValue) -> Result<(), JsValue> {
    let password: String = from_value(password)?;
    check_password(vault_name, &password).await?;

    let _lock = acquire_vault_lock(vault_name).await?;

    let root = get_root_directory_handle().await?;
    let filename = get_vault_filename(vault_name);

    let _remove_result = JsFuture::from(root.remove_entry(&filename))
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to remove vault file",
        })?;

    log(&format!("Vault file {} removed successfully", filename));
    Ok(())
}

async fn read_vault_with_name(
    vault_name: &str,
) -> Result<(FileSystemFileHandle, Vault), VaultError> {
    time_it!("Total read_vault", {
        let root = time_it!("Getting root directory", {
            get_root_directory_handle().await?
        });

        let filename = get_vault_filename(vault_name);
        let options = FileSystemGetFileOptions::new();
        let file_handle = time_it!("Getting file handle", {
            JsFuture::from(root.get_file_handle_with_options(&filename, &options))
                .await
                .map_err(|_| VaultError::IoError {
                    message: "Failed to get file handle",
                })?
                .unchecked_into::<FileSystemFileHandle>()
        });

        let file = time_it!("Getting file", {
            JsFuture::from(file_handle.get_file())
                .await
                .map_err(|_| VaultError::IoError {
                    message: "Failed to get file",
                })?
        });

        let file = web_sys::File::from(file);

        // If the file is empty, create a new vault with a fresh salt.
        if file.size() == 0f64 {
            let mut salt = [0u8; 32];
            OsRng.fill_bytes(&mut salt);
            return Ok((
                file_handle,
                Vault {
                    namespaces: HashMap::new(),
                    salt,
                },
            ));
        }

        // Read the file's data into memory
        let array_buffer = time_it!("Reading array buffer", {
            JsFuture::from(file.array_buffer())
                .await
                .map_err(|_| VaultError::IoError {
                    message: "Failed to get array buffer",
                })?
        });

        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        let bytes = uint8_array.to_vec();

        log(&format!("Read vault data size: {} bytes", bytes.len()));

        // Deserialize the vault from JSON
        let vault = time_it!("Deserializing vault", {
            serde_json::from_slice(&bytes).map_err(|e| {
                log(&format!("Deserialization error: {:?}", e));
                VaultError::SerializationError {
                    message: "Failed to deserialize vault",
                }
            })?
        });

        let (file_handle, mut vault) = (file_handle, vault);

        check_and_cleanup(&mut vault, &file_handle).await?;

        Ok((file_handle, vault))
    })
}

async fn save_vault(
    file_handle: &FileSystemFileHandle,
    mut vault: Vault,
) -> Result<(), VaultError> {
    let vault_name = file_handle.name();
    let _lock = acquire_vault_lock(&vault_name).await?;

    if let Some(_perf) = get_performance() {
        time("save_vault");
    }

    let cleanup_future: Pin<Box<dyn Future<Output = Result<(), VaultError>>>> =
        Box::pin(check_and_cleanup(&mut vault, file_handle));
    cleanup_future.await?;

    let result = time_it!("Total save_vault", {
        let json_bytes = time_it!("Serializing vault", {
            serde_json::to_vec(&vault).map_err(|_| VaultError::SerializationError {
                message: "Failed to serialize vault to JSON",
            })?
        });

        let writable = time_it!("Creating writable", {
            JsFuture::from(file_handle.create_writable())
                .await
                .map_err(|_| VaultError::IoError {
                    message: "Failed to create writable",
                })?
        });

        let write_method = js_sys::Reflect::get(&writable, &"write".into())?;
        let write_fn = write_method
            .dyn_ref::<js_sys::Function>()
            .ok_or(VaultError::IoError {
                message: "Failed to get write function",
            })?;

        time_it!("Writing data", {
            let uint8_array = js_sys::Uint8Array::from(&json_bytes[..]);
            let write_promise = write_fn.call1(&writable, &uint8_array)?;
            JsFuture::from(write_promise.unchecked_into::<js_sys::Promise>()).await?
        });

        log(&format!(
            "Writing vault data size: {} bytes",
            json_bytes.len()
        ));

        let close_val = js_sys::Reflect::get(&writable, &"close".into())?;
        let close_fn = close_val
            .dyn_ref::<js_sys::Function>()
            .ok_or(VaultError::IoError {
                message: "Failed to convert close to function",
            })?;

        let promise = close_fn.call0(&writable)?;
        JsFuture::from(promise.unchecked_into::<js_sys::Promise>()).await?;

        Ok(())
    });

    if get_performance().is_some() {
        timeEnd("save_vault");
    }

    result
}

async fn cleanup_expired_data(
    vault: &mut Vault,
    file_handle: &FileSystemFileHandle,
) -> Result<bool, VaultError> {
    let vault_name = file_handle.name();
    let _lock = acquire_vault_lock(&vault_name).await?;

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
        vault.namespaces.remove(&namespace);
        data_removed = true;
        log(&format!("Removed expired namespace: {}", namespace));
    }

    Ok(data_removed)
}

async fn check_and_cleanup(
    vault: &mut Vault,
    file_handle: &FileSystemFileHandle,
) -> Result<(), VaultError> {
    let interval = CLEANUP_INTERVAL.load(Ordering::SeqCst);
    if interval > 0 {
        let now = js_sys::Date::now() as i64 / 1000;
        let last = LAST_CLEANUP.load(Ordering::SeqCst);

        if now - last >= interval {
            let vault_name = file_handle.name();
            let _lock = acquire_vault_lock(&vault_name).await?; // Acquire lock before cleanup

            if cleanup_expired_data(vault, file_handle).await? {
                // No need to acquire lock here as save_vault already does it
                save_vault(file_handle, vault.clone()).await?;
                LAST_CLEANUP.store(now, Ordering::SeqCst);
            }
        }
    }
    Ok(())
}

fn get_vault_filename(vault_name: &str) -> String {
    format!("vault_{}.dat", vault_name)
}

#[wasm_bindgen]
pub async fn list_vaults() -> Result<JsValue, JsValue> {
    let root = get_root_directory_handle().await?;

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
                if name.starts_with("vault_") && name.ends_with(".dat") {
                    let vault_name = name
                        .trim_start_matches("vault_")
                        .trim_end_matches(".dat")
                        .to_string();
                    vault_names.push(vault_name);
                }
            }
        }
    }

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
    let vault_name: String = from_value(vault_name)?;
    validate_vault_name(&vault_name)?;

    let _lock = acquire_vault_lock(&vault_name).await?;

    if (read_vault_with_name(&vault_name).await).is_ok() {
        return Err(VaultError::VaultAlreadyExists.into());
    }

    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;

    let filename = get_vault_filename(&vault_name);
    let file_handle = get_or_create_file_handle_with_name(&filename).await?;
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);

    let mut vault = Vault {
        namespaces: HashMap::new(),
        salt,
    };

    insert_namespace_data(&mut vault, password, namespace, data, expires_in_seconds).await?;
    save_vault(&file_handle, vault).await?;

    Ok(())
}

async fn check_password(vault_name: &str, password: &str) -> Result<Vault, VaultError> {
    let (_, vault) = match read_vault_with_name(vault_name).await {
        Ok(result) => result,
        Err(VaultError::IoError { .. }) => {
            return Err(VaultError::IoError {
                message: "Vault not found",
            });
        }
        Err(e) => return Err(e),
    };

    if let Some((_, first_encrypted)) = vault.namespaces.iter().next() {
        let key_bytes = derive_key(password.as_bytes(), &vault.salt)?;
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
        log(&format!("Serialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to serialize vault for export",
        }
    })?;

    let total_size = magic.len() + 4 + serialized.len();
    let mut vault_bytes = Vec::with_capacity(total_size);

    vault_bytes.extend_from_slice(magic);
    vault_bytes.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
    vault_bytes.extend_from_slice(&serialized);

    log(&format!(
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
pub async fn import_vault(
    vault_name: &str,
    password: JsValue,
    data: JsValue,
) -> Result<(), JsValue> {
    let password: String = from_value(password)?;

    let vault_bytes = if data.is_instance_of::<js_sys::Uint8Array>() {
        let array = js_sys::Uint8Array::from(data);
        array.to_vec()
    } else {
        from_value(data)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert input data: {:?}", e)))?
    };

    log(&format!(
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
        log(&format!("Deserialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to deserialize vault data",
        }
    })?;

    check_password(vault_name, &password).await?;

    let file_handle = get_or_create_file_handle_with_name(vault_name).await?;
    save_vault(&file_handle, imported_vault).await?;

    Ok(())
}

static CLEANUP_INTERVAL: AtomicI64 = AtomicI64::new(0);
static LAST_CLEANUP: AtomicI64 = AtomicI64::new(0);

#[wasm_bindgen]
pub fn configure_cleanup(interval_seconds: i64) {
    if interval_seconds > 0 {
        log(&format!(
            "Configuring cleanup with interval of {} seconds",
            interval_seconds
        ));
        CLEANUP_INTERVAL.store(interval_seconds, Ordering::SeqCst);
        LAST_CLEANUP.store(js_sys::Date::now() as i64 / 1000, Ordering::SeqCst);
    } else {
        log("Disabling automatic cleanup");
        CLEANUP_INTERVAL.store(0, Ordering::SeqCst);
    }
}

#[wasm_bindgen]
pub async fn force_cleanup_vault(vault_name: &str) -> Result<(), JsValue> {
    let _lock = acquire_vault_lock(vault_name).await?; // Acquire lock before cleanup
    let (file_handle, mut vault) = read_vault_with_name(vault_name).await?;
    cleanup_expired_data(&mut vault, &file_handle).await?;
    Ok(())
}

async fn insert_namespace_data(
    vault: &mut Vault,
    password: JsValue,
    namespace: JsValue,
    data: JsValue,
    expires_in_seconds: Option<i64>,
) -> Result<(), JsValue> {
    let password: String = from_value(password)?;
    let namespace: String = from_value(namespace)?;

    let data_json = from_value::<serde_json::Value>(data)?;
    let data_bytes = serde_json::to_vec(&data_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize data: {:?}", e)))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(password.as_bytes(), &vault.salt)?;
    let cipher_key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(cipher_key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted_data = cipher
        .encrypt(nonce, data_bytes.as_ref())
        .map_err(|e| JsValue::from_str(&format!("Encryption failed: {:?}", e)))?;

    let expiration = expires_in_seconds.map(|seconds| {
        let now = js_sys::Date::now() as i64 / 1000;
        Expiration {
            expires_at: now + seconds,
        }
    });

    if let Some(exp) = &expiration {
        log(&format!(
            "Setting expiration for namespace '{}' to timestamp {}",
            namespace, exp.expires_at
        ));
    }

    vault.namespaces.insert(
        namespace,
        EncryptedNamespace {
            data: encrypted_data,
            nonce: nonce_bytes,
            expiration,
        },
    );

    Ok(())
}
