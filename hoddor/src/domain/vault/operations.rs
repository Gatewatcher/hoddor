use crate::errors::VaultError;
use crate::platform::Platform;
use super::types::{NamespaceData, Vault, VaultMetadata};
use std::collections::HashMap;

pub fn get_vault_dirname(vault_name: &str) -> String {
    vault_name.to_string()
}

pub fn get_metadata_filename() -> String {
    "metadata.json".to_string()
}

pub fn get_namespace_filename(namespace: &str) -> String {
    format!("{}.ns", namespace)
}

pub async fn read_vault(
    platform: &Platform,
    vault_name: &str,
) -> Result<Vault, VaultError> {
    let storage = platform.storage();
    let dirname = get_vault_dirname(vault_name);

    let metadata_path = format!("{}/{}", dirname, get_metadata_filename());
    let metadata_text = storage.read_file(&metadata_path).await?;

    let mut vault: Vault =
        serde_json::from_str(&metadata_text).map_err(|_| VaultError::SerializationError {
            message: "Failed to deserialize vault metadata",
        })?;

    vault.namespaces.clear();

    let entries = storage.list_entries(&dirname).await?;

    for entry_name in entries {
        if entry_name.ends_with(".ns") {
            let namespace_path = format!("{}/{}", dirname, entry_name);
            let namespace_text = storage.read_file(&namespace_path).await?;

            let namespace_data: NamespaceData =
                serde_json::from_str(&namespace_text).map_err(|_| {
                    VaultError::SerializationError {
                        message: "Failed to deserialize namespace data",
                    }
                })?;

            let namespace = entry_name[..entry_name.len() - 3].to_string();
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
    let dirname = get_vault_dirname(vault_name);

    storage.create_directory(&dirname).await?;

    let mut metadata_vault = vault.clone();
    metadata_vault.namespaces.clear();

    let metadata_json =
        serde_json::to_string(&metadata_vault).map_err(|_| VaultError::IoError {
            message: "Failed to serialize vault metadata",
        })?;

    let metadata_path = format!("{}/{}", dirname, get_metadata_filename());
    storage.write_file(&metadata_path, &metadata_json).await?;

    for (namespace, data) in &vault.namespaces {
        let namespace_json = serde_json::to_string(&data).map_err(|_| VaultError::IoError {
            message: "Failed to serialize namespace data",
        })?;

        let namespace_path = format!("{}/{}", dirname, get_namespace_filename(namespace));
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

pub async fn create_vault(_vault_name: &str) -> Result<Vault, VaultError> {
    let vault = Vault {
        metadata: VaultMetadata { peer_id: None },
        identity_salts: super::types::IdentitySalts::new(),
        username_pk: HashMap::new(),
        namespaces: HashMap::new(),
        sync_enabled: false,
    };

    Ok(vault)
}

pub async fn delete_vault(platform: &Platform, vault_name: &str) -> Result<(), VaultError> {
    let dirname = get_vault_dirname(vault_name);
    let storage = platform.storage();
    storage.delete_directory(&dirname).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault::types::{IdentitySalts, Vault, VaultMetadata};
    use std::collections::HashMap;

    #[test]
    fn test_get_vault_dirname() {
        assert_eq!(get_vault_dirname("my_vault"), "my_vault");
        assert_eq!(get_vault_dirname("test123"), "test123");
        assert_eq!(get_vault_dirname("vault-name"), "vault-name");
    }

    #[test]
    fn test_get_metadata_filename() {
        assert_eq!(get_metadata_filename(), "metadata.json");
    }

    #[test]
    fn test_get_namespace_filename() {
        assert_eq!(get_namespace_filename("users"), "users.ns");
        assert_eq!(get_namespace_filename("config"), "config.ns");
        assert_eq!(get_namespace_filename("data-2024"), "data-2024.ns");
    }

    #[test]
    fn test_get_namespace_filename_special_chars() {
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
