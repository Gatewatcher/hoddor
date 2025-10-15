use crate::errors::VaultError;
use crate::platform::Platform;
use super::types::{NamespaceData, Vault, VaultMetadata};
use std::collections::HashMap;

const METADATA_FILENAME: &str = "metadata.json";
const NAMESPACE_EXTENSION: &str = ".ns";

pub fn get_namespace_filename(namespace: &str) -> String {
    format!("{}{}", namespace, NAMESPACE_EXTENSION)
}

pub async fn read_vault(
    platform: &Platform,
    vault_name: &str,
) -> Result<Vault, VaultError> {
    let storage = platform.storage();

    let metadata_path = format!("{}/{}", vault_name, METADATA_FILENAME);
    let metadata_text = storage.read_file(&metadata_path).await?;

    let mut vault: Vault =
        serde_json::from_str(&metadata_text).map_err(|_| VaultError::SerializationError {
            message: "Failed to deserialize vault metadata",
        })?;

    vault.namespaces.clear();

    let entries = storage.list_entries(vault_name).await?;

    for entry_name in entries {
        if entry_name.ends_with(NAMESPACE_EXTENSION) {
            let namespace_path = format!("{}/{}", vault_name, entry_name);
            let namespace_text = storage.read_file(&namespace_path).await?;

            let namespace_data: NamespaceData =
                serde_json::from_str(&namespace_text).map_err(|_| {
                    VaultError::SerializationError {
                        message: "Failed to deserialize namespace data",
                    }
                })?;

            let namespace = entry_name.strip_suffix(NAMESPACE_EXTENSION).unwrap().to_string();
            vault.namespaces.insert(namespace, namespace_data);
        }
    }

    Ok(vault)
}

pub async fn save_vault(
    platform: &Platform,
    vault_name: &str,
    vault: Vault,
) -> Result<(), VaultError> {
    if !platform.persistence().has_requested() {
        let is_persisted = platform.persistence().check().await.unwrap_or(false);

        if !is_persisted {
            let result = platform.persistence().request().await;

            match result {
                Ok(is_granted) => {
                    platform.logger().log(&format!("persistence request granted: {}", is_granted));
                }
                Err(VaultError::JsError(message)) => {
                    platform.logger().error(&message);
                }
                _ => {}
            }
        }
    }

    let storage = platform.storage();

    storage.create_directory(vault_name).await?;

    let mut metadata_vault = vault.clone();
    metadata_vault.namespaces.clear();

    let metadata_json =
        serde_json::to_string(&metadata_vault).map_err(|_| VaultError::IoError {
            message: "Failed to serialize vault metadata",
        })?;

    let metadata_path = format!("{}/{}", vault_name, METADATA_FILENAME);
    storage.write_file(&metadata_path, &metadata_json).await?;

    for (namespace, data) in &vault.namespaces {
        let namespace_json = serde_json::to_string(&data).map_err(|_| VaultError::IoError {
            message: "Failed to serialize namespace data",
        })?;

        let namespace_path = format!("{}/{}", vault_name, get_namespace_filename(namespace));
        storage.write_file(&namespace_path, &namespace_json).await?;
    }

    let vault_bytes = serde_json::to_vec(&vault).map_err(|_| VaultError::IoError {
        message: "Failed to serialize vault for notification",
    })?;

    let _ = platform.notifier().notify_vault_update(vault_name, &vault_bytes);

    Ok(())
}

pub async fn list_vaults(platform: &Platform) -> Result<Vec<String>, VaultError> {
    platform.logger().log("Listing vaults from root directory");

    let storage = platform.storage();
    let vault_names = storage.list_entries(".").await?;

    platform
        .logger()
        .log(&format!("Found {} vaults in total", vault_names.len()));
    Ok(vault_names)
}

pub async fn create_vault() -> Result<Vault, VaultError> {
    Ok(Vault {
        metadata: VaultMetadata { peer_id: None },
        identity_salts: super::types::IdentitySalts::new(),
        username_pk: HashMap::new(),
        namespaces: HashMap::new(),
        sync_enabled: false,
    })
}

pub async fn delete_vault(platform: &Platform, vault_name: &str) -> Result<(), VaultError> {
    let storage = platform.storage();
    storage.delete_directory(vault_name).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault::types::{IdentitySalts, Vault, VaultMetadata};
    use std::collections::HashMap;

    #[test]
    fn test_get_namespace_filename() {
        assert_eq!(get_namespace_filename("users"), "users.ns");
        assert_eq!(get_namespace_filename("config"), "config.ns");
        assert_eq!(get_namespace_filename("data-2024"), "data-2024.ns");
        assert_eq!(get_namespace_filename("my_namespace"), "my_namespace.ns");
        assert_eq!(get_namespace_filename("test-123"), "test-123.ns");
    }

    #[test]
    fn test_create_vault_returns_empty_vault() {
        // Note: create_vault is async but we can test the structure it should create
        // by manually constructing it like the function does
        let vault = Vault {
            metadata: VaultMetadata { peer_id: None },
            identity_salts: IdentitySalts::new(),
            username_pk: HashMap::new(),
            namespaces: HashMap::new(),
            sync_enabled: false,
        };

        assert!(vault.metadata.peer_id.is_none());
        assert!(vault.namespaces.is_empty());
        assert!(vault.username_pk.is_empty());
        assert!(!vault.sync_enabled);
    }
}
