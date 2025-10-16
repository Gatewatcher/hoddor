/// Legacy helper functions for vault operations
/// These functions provide simple wrappers around domain operations for backward compatibility

use crate::domain::vault::{error::VaultError, NamespaceData, Vault};
use crate::domain::vault::operations::create_vault_from_sync;
use crate::platform::Platform;
use crate::sync::{OperationType, SyncMessage};
use wasm_bindgen::prelude::*;

/// Read a vault by name
pub async fn read_vault_with_name(vault_name: &str) -> Result<Vault, VaultError> {
    let platform = Platform::new();
    crate::domain::vault::operations::read_vault(&platform, vault_name).await
}

/// Save a vault with the given name
pub async fn save_vault(vault_name: &str, vault: Vault) -> Result<(), VaultError> {
    let platform = Platform::new();
    crate::domain::vault::operations::save_vault(&platform, vault_name, vault).await
}

/// Update vault from sync message
pub async fn update_vault_from_sync(vault_name: &str, vault_data: &[u8]) -> Result<(), VaultError> {
    let platform = Platform::new();

    let sync_msg: SyncMessage = serde_json::from_slice(vault_data)
        .map_err(|e| VaultError::serialization_error(format!("Failed to deserialize sync message: {:?}", e)))?;

    let mut current_vault = match read_vault_with_name(vault_name).await {
        Ok(vault) => vault,
        Err(VaultError::IoError(msg))
            if msg == "Failed to get directory handle" => {
            platform.logger().log(&format!("Creating new vault {} for sync", vault_name));

            let vault = create_vault_from_sync(
                sync_msg.vault_metadata,
                sync_msg.identity_salts.clone(),
                sync_msg.username_pk,
            ).await?;

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

/// Configure automatic cleanup of expired data
///
/// **Deprecated**: This function is a no-op stub for backward compatibility.
/// Cleanup is now handled automatically on read operations.
#[wasm_bindgen]
#[deprecated(note = "Cleanup is now automatic on read. This function does nothing.")]
pub fn configure_cleanup(_interval_seconds: i64) {
    // No-op for backward compatibility
    // Cleanup now happens automatically when reading expired data
}
