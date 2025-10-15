use crate::crypto::{
    decrypt_with_identity, encrypt_with_recipients, identity_from_passphrase, IdentityHandle,
};
use crate::errors::VaultError;
use crate::measure::time_it;
use crate::sync::{get_sync_manager, OperationType, SyncMessage};
use crate::webrtc::{AccessLevel, WebRtcPeer};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::platform::Platform;
use core::str;
use serde_wasm_bindgen::{from_value, to_value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

use rand::RngCore;

use argon2::password_hash::rand_core::OsRng;

use futures_channel::mpsc::UnboundedReceiver;

pub use crate::domain::vault::{IdentitySalts, NamespaceData, Vault, VaultMetadata};
use crate::domain::vault::expiration::{cleanup_expired_namespaces, create_expiration, is_expired};
use crate::domain::vault::operations::get_namespace_filename;
use crate::domain::vault::validation::{validate_namespace, validate_passphrase, validate_vault_name};

#[wasm_bindgen]
pub async fn vault_identity_from_passphrase(
    passphrase: &str,
    vault_name: &str,
) -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();
    vault_identity_from_passphrase_internal(&platform, passphrase, vault_name).await
}

async fn vault_identity_from_passphrase_internal(
    platform: &Platform,
    passphrase: &str,
    vault_name: &str,
) -> Result<IdentityHandle, JsValue> {
    validate_passphrase(passphrase).map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    validate_vault_name(vault_name)?;

    let mut vault = match read_vault_with_name(vault_name).await {
        Ok(result) => result,
        Err(_) => {
            return Err(JsValue::from_str(&format!(
                "Vault '{}' does not exist",
                vault_name
            )));
        }
    };

    // Try to find an existing identity by iterating over stored salts
    for (stored_pubkey, salt) in vault.identity_salts.iter() {
        platform.logger().log(&format!("Checking stored public key: {}", stored_pubkey));

        // Validate salt length
        if salt.len() != 32 {
            platform.logger().error(&format!(
                "Invalid salt length ({}) for public key: {}",
                salt.len(),
                stored_pubkey
            ));
            continue;
        }

        platform.logger().log(&format!("Using salt: {:?}", salt));

        match identity_from_passphrase(passphrase, salt).await {
            Ok(identity) => {
                platform.logger().log(&format!("Generated public key: {}", identity.public_key()));
                if identity.public_key() == *stored_pubkey {
                    platform.logger().log("Found matching identity");
                    return Ok(identity);
                } else {
                    platform.logger().warn("Public key does not match stored salt");
                }
            }
            Err(err) => {
                platform.logger().warn(&format!(
                    "Failed to generate identity with stored salt for public key {}: {:?}",
                    stored_pubkey, err
                ));
            }
        };
    }

    platform.logger().log("No matching identity found; generating new salt");
    let mut new_salt = [0u8; 32];
    OsRng.fill_bytes(&mut new_salt);

    let identity = identity_from_passphrase(passphrase, &new_salt)
        .await
        .map_err(|e| {
            platform.logger().error(&format!("Failed to create new identity: {:?}", e));
            JsValue::from_str(&format!("Failed to create new identity: {:?}", e))
        })?;

    // Store the new salt with the generated public key
    vault
        .identity_salts
        .set_salt(identity.public_key(), new_salt);

    save_vault(vault_name, vault).await.map_err(|e| {
        platform.logger().error(&format!("Failed to save vault: {:?}", e));
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
    let platform = Platform::new();

    // Validate namespace first
    validate_namespace(namespace)?;

    let mut retries = 10;
    let mut delay = 50;
    let mut last_error = None;

    while retries > 0 {
        let lock = match platform.locks().acquire(vault_name).await {
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
                Ok(v) => v,
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

            save_vault(vault_name, vault.clone()).await?;

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

    let mut vault = read_vault_with_name(vault_name).await?;

    if vault.namespaces.remove(&namespace).is_none() {
        return Err(VaultError::NamespaceNotFound.into());
    }

    let namespace_filename = get_namespace_filename(&namespace);
    let namespace_path = format!("{}/{}", vault_name, namespace_filename);

    let platform = Platform::new();
    let storage = platform.storage();
    storage.delete_file(&namespace_path).await.map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    save_vault(vault_name, vault).await?;
    Ok(())
}

#[wasm_bindgen]
pub async fn read_from_vault(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<JsValue, JsValue> {
    let platform = Platform::new();
    read_from_vault_internal(&platform, vault_name, identity, namespace).await
}

async fn read_from_vault_internal(
    platform: &Platform,
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<JsValue, JsValue> {
    let namespace_str: String = from_value(namespace.clone())?;
    validate_namespace(&namespace_str)?;
    let namespace_str = namespace.as_string().unwrap_or_default();
    if platform.clock().is_available() {
        platform.logger().time(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    let result = time_it!("Total read_from_vault", {
        let namespace: String = from_value(namespace).map_err(|_| VaultError::IoError {
            message: "Invalid namespace format",
        })?;

        let mut vault = match read_vault_with_name(vault_name).await {
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

        let now = js_sys::Date::now() as i64 / 1000;
        if is_expired(&encrypted_namespace.expiration, now) {
            vault.namespaces.remove(&namespace);
            save_vault(vault_name, vault.clone()).await?;
            return Err(VaultError::DataExpired.into());
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

    if platform.clock().is_available() {
        platform.logger().time_end(&format!("read_from_vault {} {}", vault_name, namespace_str));
    }

    result
}

#[wasm_bindgen]
pub async fn list_namespaces(vault_name: &str) -> Result<JsValue, JsValue> {
    let platform = Platform::new();
    list_namespaces_internal(&platform, vault_name)
        .await
        .map(|namespaces| to_value(&namespaces).unwrap())
        .map_err(|e| e.into())
}

async fn list_namespaces_internal(
    platform: &Platform,
    vault_name: &str,
) -> Result<Vec<String>, VaultError> {
    let vault = match read_vault_with_name(vault_name).await {
        Ok(result) => result,
        Err(VaultError::IoError { .. }) => {
            return Ok(Vec::new());
        }
        Err(e) => return Err(e),
    };

    platform.logger().log(&format!(
        "Found {} namespaces in vault",
        vault.namespaces.len()
    ));
    for key in vault.namespaces.keys() {
        platform.logger().log(&format!("Namespace found: {}", key));
    }

    let namespaces: Vec<String> = vault.namespaces.keys().cloned().collect();

    platform.logger().log(&format!("Returning {} namespaces", namespaces.len()));
    Ok(namespaces)
}

#[wasm_bindgen]
pub async fn remove_vault(vault_name: &str) -> Result<(), JsValue> {
    let platform = Platform::new();
    let _lock = platform.locks().acquire(vault_name).await?;

    crate::domain::vault::operations::delete_vault(&platform, vault_name)
        .await
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    Ok(())
}

#[wasm_bindgen]
pub async fn list_vaults() -> Result<JsValue, JsValue> {
    let platform = Platform::new();
    list_vaults_internal(&platform)
        .await
        .map(|vaults| to_value(&vaults).unwrap())
        .map_err(|e| e.into())
}

async fn list_vaults_internal(platform: &Platform) -> Result<Vec<String>, VaultError> {
    crate::domain::vault::operations::list_vaults(platform).await
}

#[wasm_bindgen]
pub async fn create_vault(vault_name: JsValue) -> Result<(), JsValue> {
    let name = vault_name
        .as_string()
        .ok_or_else(|| JsValue::from_str("vault_name must be a string"))?;

    create_vault_internal(&name).await
}

async fn create_vault_internal(vault_name: &str) -> Result<(), JsValue> {
    validate_vault_name(vault_name).map_err(|e| JsValue::from_str(&format!("{}", e)))?;

    if let Ok(_) = read_vault_with_name(vault_name).await {
        return Err(JsValue::from_str(&format!(
            "Vault '{}' already exists",
            vault_name
        )));
    }

    let vault = crate::domain::vault::operations::create_vault()
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create vault: {:?}", e)))?;

    save_vault(vault_name, vault)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to save vault: {:?}", e)))?;

    Ok(())
}

async fn check_identity(vault_name: &str, identity: &IdentityHandle) -> Result<Vault, VaultError> {
    let vault = match read_vault_with_name(vault_name).await {
        Ok(existing_vault) => existing_vault,

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
    let platform = Platform::new();
    export_vault_internal(&platform, vault_name)
        .await
        .map_err(|e| e.into())
}

async fn export_vault_internal(
    platform: &Platform,
    vault_name: &str,
) -> Result<JsValue, VaultError> {
    let vault = read_vault_with_name(vault_name).await?;

    // Create binary format with magic number "VAULT1"
    let magic = b"VAULT1";
    let serialized = serde_json::to_vec(&vault).map_err(|e| {
        platform.logger().log(&format!("Serialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to serialize vault for export",
        }
    })?;

    let total_size = magic.len() + 4 + serialized.len();
    let mut vault_bytes = Vec::with_capacity(total_size);

    vault_bytes.extend_from_slice(magic);
    vault_bytes.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
    vault_bytes.extend_from_slice(&serialized);

    platform.logger().log(&format!(
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
    let platform = Platform::new();

    let vault_bytes = if data.is_instance_of::<js_sys::Uint8Array>() {
        let array = js_sys::Uint8Array::from(data);
        array.to_vec()
    } else {
        from_value(data)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert input data: {:?}", e)))?
    };

    import_vault_internal(&platform, vault_name, &vault_bytes)
        .await
        .map_err(|e| e.into())
}

async fn import_vault_internal(
    platform: &Platform,
    vault_name: &str,
    vault_bytes: &[u8],
) -> Result<(), VaultError> {
    platform.logger().log(&format!(
        "Attempting to import vault data of size: {} bytes",
        vault_bytes.len()
    ));

    if vault_bytes.len() < 10 || &vault_bytes[..6] != b"VAULT1" {
        return Err(VaultError::SerializationError {
            message: "Invalid vault file: missing or incorrect magic number",
        });
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
        });
    }

    let imported_vault: Vault = serde_json::from_slice(&vault_bytes[10..]).map_err(|e| {
        platform.logger().log(&format!("Deserialization error: {:?}", e));
        VaultError::SerializationError {
            message: "Failed to deserialize vault data",
        }
    })?;

    match read_vault_with_name(vault_name).await {
        Ok(_) => {
            return Err(VaultError::VaultAlreadyExists);
        }
        Err(VaultError::IoError { .. }) => {
            platform.logger().log(&format!(
                "No existing vault named '{}'; proceeding with import.",
                vault_name
            ));
        }
        Err(e) => {
            return Err(e);
        }
    };

    save_vault(vault_name, imported_vault).await?;

    Ok(())
}

pub async fn read_vault_with_name(
    vault_name: &str,
) -> Result<Vault, VaultError> {
    let platform = Platform::new();
    crate::domain::vault::operations::read_vault(&platform, vault_name).await
}

pub async fn save_vault(
    vault_name: &str,
    vault: Vault,
) -> Result<(), VaultError> {
    let platform = Platform::new();
    crate::domain::vault::operations::save_vault(&platform, vault_name, vault).await
}

async fn cleanup_expired_data(
    platform: &Platform,
    vault: &mut Vault,
    vault_name: &str,
) -> Result<bool, VaultError> {
    let now = js_sys::Date::now() as i64 / 1000;
    let data_removed = cleanup_expired_namespaces(platform, vault, vault_name, now).await?;

    if data_removed {
        save_vault(vault_name, vault.clone()).await?;
    }

    Ok(data_removed)
}

#[wasm_bindgen]
pub async fn force_cleanup_vault(vault_name: &str) -> Result<(), JsValue> {
    let platform = Platform::new();
    let _lock = platform.locks().acquire(vault_name).await?;
    let mut vault = read_vault_with_name(vault_name).await?;

    while cleanup_expired_data(&platform, &mut vault, vault_name).await? {}

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

    let now = js_sys::Date::now() as i64 / 1000;
    let expiration = create_expiration(expires_in_seconds, now);

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
    let platform = Platform::new();
    enable_sync_internal(&platform, vault_name, identity, signaling_url, stun_servers).await
}

async fn enable_sync_internal(
    platform: &Platform,
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

    let vault = updated_vault.clone();

    save_vault(vault_name, vault.clone()).await?;

    let (mut peer, _receiver): (WebRtcPeer, UnboundedReceiver<Vec<u8>>) =
        WebRtcPeer::create_peer(vault.metadata.peer_id.clone().unwrap(), stun_servers_vec).await?;

    platform.logger().log(&format!(
        "Connecting to signaling server at {}",
        signaling_url_str
    ));

    platform.logger().log("Connecting to signaling server...");
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
    let platform = Platform::new();
    connect_to_peer_internal(&platform, vault_name, identity, peer_id, signaling_url).await
}

async fn connect_to_peer_internal(
    platform: &Platform,
    vault_name: &str,
    identity: &IdentityHandle,
    peer_id: JsValue,
    signaling_url: JsValue,
) -> Result<(), JsValue> {
    platform.logger().log(&format!(
        "connect_to_peer called with: vault_name = {}",
        vault_name
    ));
    platform.logger().log(&format!("identity = {:?}", identity));
    platform.logger().log(&format!("peer_id = {:?}", peer_id));
    platform.logger().log(&format!("signaling_url = {:?}", signaling_url));

    let peer_id_str: String = from_value(peer_id)?;
    let signaling_url_str: String = from_value(signaling_url)?;

    platform.logger().log("Checking identity...");
    let vault = check_identity(vault_name, identity).await?;

    if !vault.sync_enabled {
        let msg = "Sync is not enabled for this vault";
        platform.logger().error(msg);
        return Err(JsValue::from_str(msg));
    }

    let my_peer_id = vault
        .metadata
        .peer_id
        .clone()
        .ok_or_else(|| JsValue::from_str("No peer ID found in vault metadata"))?;

    platform.logger().log("Creating WebRTC peer...");
    let stun_servers = js_sys::Array::new();
    stun_servers.push(&"stun:stun.l.google.com:19302".into());
    let stun_servers: Vec<String> = stun_servers
        .iter()
        .map(|s| s.as_string().unwrap_or_default())
        .collect();

    let (peer, _receiver): (WebRtcPeer, UnboundedReceiver<Vec<u8>>) =
        WebRtcPeer::create_peer(my_peer_id, stun_servers).await?;
    let peer_rc = Rc::new(RefCell::new(peer));

    platform.logger().log("Connecting to signaling server...");
    peer_rc
        .borrow_mut()
        .connect(&signaling_url_str, Some(&peer_id_str))
        .await?;

    platform.logger().log("Adding peer to sync manager...");
    platform.logger().log(&format!("Adding peer {} to sync manager", peer_id_str));
    let sync_manager = get_sync_manager(vault_name)?;
    sync_manager.borrow_mut().add_peer(peer_rc);

    platform.logger().log("Sending initial vault data to the peer...");
    let vault = read_vault_with_name(vault_name).await?;
    let sync_manager = get_sync_manager(vault_name)?;
    let mut sync_manager = sync_manager.borrow_mut();

    let mut operations = Vec::new();
    platform.logger().log(&format!(
        "Found {} namespaces to sync",
        vault.namespaces.len()
    ));

    for (namespace, data) in vault.namespaces {
        platform.logger().log(&format!("Creating operation for namespace: {}", namespace));
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
        platform.logger().log(&format!(
            "Waiting for WebRTC connection to be ready (attempt {})",
            retries + 1
        ));
        platform.logger().log(&format!(
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
        platform.logger().log(&format!(
            "Sending sync message for vault: {}, namespace: {}",
            vault_name, sync_msg.operation.namespace
        ));

        let msg_bytes = serde_json::to_vec(&sync_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize sync message: {}", e)))?;

        peer_ref.send_message(msg_bytes)?;
        platform.logger().log("Sync message sent successfully");
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
    let platform = Platform::new();
    add_peer_internal(&platform, vault_name, identity, peer_id, namespace, access_level).await
}

async fn add_peer_internal(
    platform: &Platform,
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
        platform.logger().log(&format!(
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
            platform.logger().log(&format!(
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

        platform.logger().log(&format!(
            "Sending data for namespace {} to peer",
            namespace_str
        ));

        let msg_bytes = serde_json::to_vec(&sync_msg)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize sync message: {}", e)))?;

        peer_ref.send_message(msg_bytes)?;
        platform.logger().log("Data sent successfully");
    } else {
        platform.logger().log(&format!("No data found for namespace {}", namespace_str));
    }

    Ok(())
}

#[wasm_bindgen]
pub async fn update_vault_from_sync(vault_name: &str, vault_data: &[u8]) -> Result<(), VaultError> {
    let platform = Platform::new();
    update_vault_from_sync_internal(&platform, vault_name, vault_data).await
}

async fn update_vault_from_sync_internal(
    platform: &Platform,
    vault_name: &str,
    vault_data: &[u8],
) -> Result<(), VaultError> {
    let sync_msg: SyncMessage = serde_json::from_slice(vault_data)
        .map_err(|e| VaultError::JsError(format!("Failed to deserialize sync message: {:?}", e)))?;

    let mut current_vault = match read_vault_with_name(vault_name).await {
        Ok(vault) => vault,
        Err(VaultError::IoError {
            message: "Failed to get directory handle",
        }) => {
            platform.logger().log(&format!("Creating new vault {} for sync", vault_name));

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

            save_vault(vault_name, vault.clone()).await?;

            vault
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
                platform.logger().log(&format!("Updated namespace {} in vault", namespace));
            }
        }
        OperationType::Delete => {
            let namespace = sync_msg.operation.namespace.clone();
            current_vault.namespaces.remove(&namespace);
            platform.logger().log(&format!("Removed namespace {} from vault", namespace));
        }
    }

    save_vault(vault_name, current_vault).await?;

    Ok(())
}

#[wasm_bindgen]
pub fn configure_cleanup(interval_seconds: i64) {
    let platform = Platform::new();
    configure_cleanup_internal(&platform, interval_seconds);
}

fn configure_cleanup_internal(platform: &Platform, interval_seconds: i64) {
    if interval_seconds > 0 {
        platform.logger().log(&format!(
            "Configuring cleanup with interval of {} seconds",
            interval_seconds
        ));
        CLEANUP_INTERVAL.store(interval_seconds, Ordering::SeqCst);
        LAST_CLEANUP.store(js_sys::Date::now() as i64 / 1000, Ordering::SeqCst);
    } else {
        platform.logger().log("Disabling automatic cleanup");
        CLEANUP_INTERVAL.store(0, Ordering::SeqCst);
    }
}
