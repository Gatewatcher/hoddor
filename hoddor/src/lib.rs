extern crate console_error_panic_hook;

use core::str;
use js_sys::{Function, Reflect};
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::HashMap, fmt};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};

use web_sys::{
    self, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions, Lock,
    LockManager, LockOptions, Performance, WorkerGlobalScope,
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

use rand::RngCore;
use serde_json;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EncryptedNamespace {
    data: Vec<u8>,
    nonce: [u8; 12],
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Vault {
    namespaces: HashMap<String, EncryptedNamespace>,
    salt: [u8; 32],
}

#[derive(Debug)]
enum VaultError {
    IoError { message: &'static str },
    NamespaceNotFound,
    InvalidPassword,
    SerializationError { message: &'static str },
    JsError(String),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::IoError { message } => write!(f, "IO Error: {}", message),
            VaultError::NamespaceNotFound => write!(f, "Namespace not found"),
            VaultError::InvalidPassword => write!(f, "Invalid password"),
            VaultError::SerializationError { message } => {
                write!(f, "Serialization Error: {}", message)
            }
            VaultError::JsError(msg) => write!(f, "JavaScript Error: {}", msg),
        }
    }
}

impl From<JsValue> for VaultError {
    fn from(err: JsValue) -> Self {
        VaultError::JsError(
            err.as_string()
                .unwrap_or_else(|| "Unknown JS error".to_string()),
        )
    }
}

impl From<VaultError> for JsValue {
    fn from(error: VaultError) -> Self {
        JsValue::from_str(&error.to_string())
    }
}

#[derive(Debug)]
enum LockError {
    AcquisitionFailed,
}

impl From<LockError> for VaultError {
    fn from(error: LockError) -> Self {
        match error {
            LockError::AcquisitionFailed => VaultError::IoError {
                message: "Failed to acquire lock",
            },
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn time(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn timeEnd(s: &str);
}

static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

#[wasm_bindgen]
pub fn set_debug_mode(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::SeqCst);
}

fn get_global_scope() -> Result<JsValue, VaultError> {
    // Try worker scope first
    if let Ok(scope) = js_sys::global().dyn_into::<WorkerGlobalScope>() {
        return Ok(JsValue::from(scope));
    }

    // Fallback to window
    let window = web_sys::window().ok_or(VaultError::IoError {
        message: "Neither WorkerGlobalScope nor Window found",
    })?;
    Ok(JsValue::from(window))
}

fn get_performance() -> Option<Performance> {
    match get_global_scope() {
        Ok(scope) => {
            if let Ok(worker) = scope.clone().dyn_into::<WorkerGlobalScope>() {
                worker.performance()
            } else if let Ok(window) = scope.dyn_into::<web_sys::Window>() {
                window.performance()
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

macro_rules! time_it {
    ($label:expr, $block:expr) => {{
        let debug = DEBUG_MODE.load(Ordering::SeqCst);
        if debug {
            if let Some(_) = get_performance() {
                time($label);
            }
        }
        let result = $block;
        if debug {
            if let Some(_) = get_performance() {
                timeEnd($label);
            }
        }
        result
    }};
}

#[wasm_bindgen(start)]
pub fn start() {
    // Set a panic hook for clearer errors in the console.
    console_error_panic_hook::set_once();
    log("Worker started (File System Access API assumed available).");
}

#[wasm_bindgen]
pub fn hash_password(password: JsValue) -> Result<JsValue, JsValue> {
    let serde_password: String = from_value(password)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize password: {:?}", e)))?;

    let argon = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon
        .hash_password(serde_password.as_bytes(), &salt)
        .map_err(|e| JsValue::from_str(&format!("Failed to hash password: {:?}", e)))?
        .to_string();

    Ok(to_value(&password_hash)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize hash: {:?}", e)))?)
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
pub async fn create_vault(
    password: JsValue,
    namespace: JsValue,
    data: JsValue,
) -> Result<(), JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    create_vault_with_name(JsValue::from_str("default"), password, namespace, data).await
}

#[wasm_bindgen]
pub async fn upsert_vault(
    password: JsValue,
    namespace: JsValue,
    data: JsValue,
) -> Result<(), JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    upsert_vault_with_name("default", password, namespace, data).await
}

#[wasm_bindgen]
pub async fn upsert_vault_with_name(
    vault_name: &str,
    password: JsValue,
    namespace: JsValue,
    data: JsValue,
) -> Result<(), JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    let password: String = from_value(password)?;
    let namespace: String = from_value(namespace)?;

    let data_json = from_value::<serde_json::Value>(data)?;
    let data_bytes = serde_json::to_vec(&data_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize data: {:?}", e)))?;

    let (file_handle, mut vault) = match read_vault_with_name(vault_name).await {
        Ok((handle, existing_vault)) => {
            log("Vault file found. Will add/update the existing vault...");
            (handle, existing_vault)
        }
        Err(VaultError::IoError { .. }) => {
            log("No existing vault found. Creating a new vault...");
            let mut salt = [0u8; 32];
            OsRng.fill_bytes(&mut salt);

            let vault = Vault {
                namespaces: HashMap::new(),
                salt,
            };
            let file_handle = get_or_create_file_handle_with_name(vault_name).await?;
            (file_handle, vault)
        }
        Err(e) => return Err(e.into()),
    };

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(password.as_bytes(), &vault.salt)?;
    let cipher_key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(cipher_key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted_data = cipher
        .encrypt(nonce, data_bytes.as_ref())
        .map_err(|e| JsValue::from_str(&format!("Encryption failed: {:?}", e)))?;

    vault.namespaces.insert(
        namespace,
        EncryptedNamespace {
            data: encrypted_data,
            nonce: nonce_bytes,
        },
    );

    save_vault(&file_handle, &vault)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(())
}

#[wasm_bindgen]
pub async fn remove_from_vault(password: JsValue, namespace: JsValue) -> Result<(), JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    remove_from_vault_with_name("default", password, namespace).await
}

#[wasm_bindgen]
pub async fn remove_from_vault_with_name(
    vault_name: &str,
    password: JsValue,
    namespace: JsValue,
) -> Result<(), JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    let password: String = from_value(password)?;
    let namespace: String = from_value(namespace)?;

    let (file_handle, mut vault) = read_vault_with_name(vault_name).await?;

    let key_bytes = derive_key(password.as_bytes(), &vault.salt)?;
    let cipher_key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(cipher_key);

    if let Some((_, sample_enc)) = vault.namespaces.iter().next() {
        let nonce = Nonce::from_slice(&sample_enc.nonce);
        time_it!("Password verification decryption", {
            cipher
                .decrypt(nonce, sample_enc.data.as_ref())
                .map_err(|_| VaultError::InvalidPassword)?
        });
    }

    if !vault.namespaces.remove(&namespace).is_some() {
        return Err(VaultError::NamespaceNotFound.into());
    }

    save_vault(&file_handle, &vault).await?;
    Ok(())
}

#[wasm_bindgen]
pub async fn read_from_vault(password: JsValue, namespace: JsValue) -> Result<JsValue, JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    read_from_vault_with_name("default", password, namespace).await
}

#[wasm_bindgen]
pub async fn read_from_vault_with_name(
    vault_name: &str,
    password: JsValue,
    namespace: JsValue,
) -> Result<JsValue, JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    let namespace_str = namespace.as_string().unwrap_or_default();
    if let Some(_) = get_performance() {
        time(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    let result = time_it!("Total read_from_vault", {
        let password: String = from_value(password).map_err(|_| VaultError::IoError {
            message: "Invalid password format",
        })?;
        let namespace: String = from_value(namespace).map_err(|_| VaultError::IoError {
            message: "Invalid namespace format",
        })?;

        let (_, vault) = match read_vault_with_name(vault_name).await {
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

    if let Some(_) = get_performance() {
        timeEnd(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    result
}

#[wasm_bindgen]
pub async fn list_namespaces(password: JsValue) -> Result<JsValue, JsValue> {
    list_namespaces_with_name("default", password).await
}

#[wasm_bindgen]
pub async fn list_namespaces_with_name(
    vault_name: &str,
    password: JsValue,
) -> Result<JsValue, JsValue> {
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

    let key_bytes = derive_key(password.as_bytes(), &vault.salt)?;
    let cipher_key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(cipher_key);

    if let Some((_, first_encrypted)) = vault.namespaces.iter().next() {
        time_it!("Password verification decryption", {
            cipher
                .decrypt(
                    Nonce::from_slice(&first_encrypted.nonce),
                    first_encrypted.data.as_ref(),
                )
                .map_err(|_| VaultError::InvalidPassword)?
        });
    }

    let namespaces: Vec<String> = vault.namespaces.keys().cloned().collect();

    log(&format!("Returning {} namespaces", namespaces.len()));
    Ok(to_value(&namespaces)?)
}

#[wasm_bindgen]
pub async fn remove_vault() -> Result<(), JsValue> {
    remove_vault_with_name("default").await
}

#[wasm_bindgen]
pub async fn remove_vault_with_name(vault_name: &str) -> Result<(), JsValue> {
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

fn derive_key(password: &[u8], salt: &[u8]) -> Result<[u8; 32], JsValue> {
    time_it!("derive_key", {
        let argon2 = Argon2::default();
        let mut key = [0u8; 32];

        argon2
            .hash_password_into(password, salt, &mut key)
            .map_err(|e| JsValue::from_str(&format!("Key derivation failed: {:?}", e)))?;

        Ok(key)
    })
}

async fn read_vault_with_name(
    vault_name: &str,
) -> Result<(FileSystemFileHandle, Vault), VaultError> {
    // Removed lock here since this is a read operation
    let result = time_it!("Total read_vault", {
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

        Ok((file_handle, vault))
    });

    result
}

async fn save_vault(file_handle: &FileSystemFileHandle, vault: &Vault) -> Result<(), VaultError> {
    // Keep lock for write operations
    let vault_name = file_handle.name();
    let _lock = acquire_vault_lock(&vault_name).await?;

    if let Some(_perf) = get_performance() {
        time("save_vault");
    }

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

        let write_method = Reflect::get(&writable, &"write".into())?;
        let write_fn = write_method
            .dyn_ref::<Function>()
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

        let close_val = Reflect::get(&writable, &"close".into())?;
        let close_fn = close_val.dyn_ref::<Function>().ok_or(VaultError::IoError {
            message: "Failed to convert close to function",
        })?;

        let promise = close_fn.call0(&writable)?;
        JsFuture::from(promise.unchecked_into::<js_sys::Promise>()).await?;

        Ok(())
    });

    if let Some(_) = get_performance() {
        timeEnd("save_vault");
    }

    result
}

async fn get_root_directory_handle() -> Result<FileSystemDirectoryHandle, VaultError> {
    let global = get_global_scope()?;

    let storage = if let Ok(worker) = global.clone().dyn_into::<WorkerGlobalScope>() {
        worker.navigator().storage()
    } else if let Ok(window) = global.dyn_into::<web_sys::Window>() {
        window.navigator().storage()
    } else {
        return Err(VaultError::IoError {
            message: "Could not access navigator",
        });
    };

    let dir_promise = storage.get_directory();
    let dir_handle = JsFuture::from(dir_promise)
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to get directory",
        })?
        .unchecked_into::<FileSystemDirectoryHandle>();

    Ok(dir_handle)
}

async fn get_or_create_file_handle_with_name(
    vault_name: &str,
) -> Result<FileSystemFileHandle, VaultError> {
    let root = get_root_directory_handle().await?;
    let filename = get_vault_filename(vault_name);
    let options = FileSystemGetFileOptions::new();
    options.set_create(true);

    let file_handle = JsFuture::from(root.get_file_handle_with_options(&filename, &options))
        .await
        .map_err(|_| VaultError::IoError {
            message: "Failed to get or create file handle",
        })?
        .unchecked_into::<FileSystemFileHandle>();

    Ok(file_handle)
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

        if let Some(value) = js_sys::Reflect::get(&next_result, &JsValue::from_str("value"))?
            .dyn_into::<js_sys::Array>()
            .ok()
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

async fn get_lock_manager() -> Result<LockManager, VaultError> {
    let global = get_global_scope()?;

    if let Ok(worker) = global.clone().dyn_into::<WorkerGlobalScope>() {
        Ok(worker.navigator().locks())
    } else if let Ok(window) = global.dyn_into::<web_sys::Window>() {
        Ok(window.navigator().locks())
    } else {
        Err(VaultError::IoError {
            message: "Could not access navigator",
        })
    }
}

async fn acquire_vault_lock(vault_name: &str) -> Result<Lock, VaultError> {
    let lock_manager = get_lock_manager().await?;
    let lock_name = format!("vault_{}_lock", vault_name);

    let options = LockOptions::new();
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("mode"),
        &JsValue::from_str("exclusive"),
    )?;
    options.set_if_available(false);

    let callback = Closure::wrap(Box::new(|| {}) as Box<dyn Fn()>);
    let promise = lock_manager.request_with_options_and_callback(
        &lock_name,
        &options,
        callback.as_ref().unchecked_ref(),
    );
    let lock = JsFuture::from(promise)
        .await
        .map_err(|_| LockError::AcquisitionFailed)?;

    callback.forget(); // Prevent the callback from being dropped
    Ok(lock.unchecked_into::<Lock>())
}

#[wasm_bindgen]
pub async fn create_vault_with_name(
    vault_name: JsValue,
    password: JsValue,
    namespace: JsValue,
    data: JsValue,
) -> Result<(), JsValue> {
    let vault_name: String = from_value(vault_name)?;
    validate_vault_name(&vault_name)?;

    let _lock = acquire_vault_lock(&vault_name).await?;
    upsert_vault_with_name(&vault_name, password, namespace, data).await
}

#[wasm_bindgen]
pub async fn export_vault(password: JsValue) -> Result<JsValue, JsValue> {
    export_vault_with_name("default", password).await
}

#[wasm_bindgen]
pub async fn export_vault_with_name(
    vault_name: &str,
    password: JsValue,
) -> Result<JsValue, JsValue> {
    let password: String = from_value(password)?;

    let (_, vault) = match read_vault_with_name(vault_name).await {
        Ok(result) => result,
        Err(VaultError::IoError { .. }) => {
            return Err(VaultError::IoError {
                message: "Vault not found",
            }
            .into());
        }
        Err(e) => return Err(e.into()),
    };

    // Verify the password
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

    // Serialize to binary format
    let mut vault_bytes = Vec::new();
    let magic = b"VAULT10"; // Magic number + version
    vault_bytes.extend_from_slice(magic);

    let serialized = serde_json::to_vec(&vault).map_err(|_| VaultError::SerializationError {
        message: "Failed to serialize vault for export",
    })?;

    let length = serialized.len() as u32;
    vault_bytes.extend_from_slice(&length.to_be_bytes());

    vault_bytes.extend_from_slice(&serialized);

    Ok(to_value(&vault_bytes)?)
}

#[wasm_bindgen]
pub async fn import_vault(password: JsValue, data: JsValue) -> Result<(), JsValue> {
    import_vault_with_name("default", password, data).await
}

#[wasm_bindgen]
pub async fn import_vault_with_name(
    vault_name: &str,
    password: JsValue,
    data: JsValue,
) -> Result<(), JsValue> {
    let password: String = from_value(password)?;

    let vault_bytes: Vec<u8> = from_value(data)
        .map_err(|e| JsValue::from_str(&format!("Failed to convert input data: {:?}", e)))?;

    log(&format!(
        "Attempting to import vault data of size: {} bytes",
        vault_bytes.len()
    ));

    let length = u32::from_be_bytes([
        vault_bytes[6],
        vault_bytes[7],
        vault_bytes[8],
        vault_bytes[9],
    ]) as usize;

    if vault_bytes.len() < length + 10 {
        return Err(VaultError::SerializationError {
            message: "Invalid vault file: truncated data",
        }
        .into());
    }

    let content = &vault_bytes[10..];
    let imported_vault: Vault = serde_json::from_slice(content).map_err(|e| {
        log(&format!("Binary deserialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to deserialize vault data",
        }
    })?;

    if let Some((_, first_encrypted)) = imported_vault.namespaces.iter().next() {
        let key_bytes = derive_key(password.as_bytes(), &imported_vault.salt)?;
        let cipher_key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(cipher_key);

        cipher
            .decrypt(
                Nonce::from_slice(&first_encrypted.nonce),
                first_encrypted.data.as_ref(),
            )
            .map_err(|_| VaultError::InvalidPassword)?;
    }

    let file_handle = get_or_create_file_handle_with_name(vault_name).await?;
    save_vault(&file_handle, &imported_vault).await?;

    Ok(())
}
