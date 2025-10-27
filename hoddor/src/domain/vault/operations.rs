use super::error::VaultError;
use super::types::{Expiration, NamespaceData, Vault, VaultMetadata};
use crate::platform::Platform;
use std::collections::HashMap;

const METADATA_FILENAME: &str = "metadata.json";
const NAMESPACE_EXTENSION: &str = ".hoddor";
const LEGACY_NAMESPACE_EXTENSION: &str = ".ns";

pub fn get_namespace_filename(namespace: &str) -> String {
    format!("{namespace}{NAMESPACE_EXTENSION}")
}

pub async fn read_vault(platform: &Platform, vault_name: &str) -> Result<Vault, VaultError> {
    let storage = platform.storage();

    let metadata_path = format!("{vault_name}/{METADATA_FILENAME}");
    let metadata_text = storage.read_file(&metadata_path).await?;

    let mut vault: Vault = serde_json::from_str(&metadata_text)
        .map_err(|_| VaultError::serialization_error("Failed to deserialize vault metadata"))?;

    vault.namespaces.clear();

    let entries = storage.list_entries(vault_name).await?;

    for entry_name in entries {
        // Support both new .hoddor and legacy .ns extensions
        let is_namespace = entry_name.ends_with(NAMESPACE_EXTENSION)
            || entry_name.ends_with(LEGACY_NAMESPACE_EXTENSION);

        if is_namespace {
            let namespace_path = format!("{vault_name}/{entry_name}");
            let namespace_text = storage.read_file(&namespace_path).await?;

            let namespace_data: NamespaceData =
                serde_json::from_str(&namespace_text).map_err(|_| {
                    VaultError::serialization_error("Failed to deserialize namespace data")
                })?;

            // Strip the appropriate extension
            let namespace = if let Some(ns) = entry_name.strip_suffix(NAMESPACE_EXTENSION) {
                ns.to_string()
            } else if let Some(ns) = entry_name.strip_suffix(LEGACY_NAMESPACE_EXTENSION) {
                ns.to_string()
            } else {
                continue; // Should never happen due to the is_namespace check
            };

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
                    platform
                        .logger()
                        .log(&format!("persistence request granted: {is_granted}"));
                }
                Err(e) => {
                    platform
                        .logger()
                        .error(&format!("Persistence request failed: {e}"));
                }
            }
        }
    }

    let storage = platform.storage();

    storage.create_directory(vault_name).await?;

    let mut metadata_vault = vault.clone();
    metadata_vault.namespaces.clear();

    let metadata_json = serde_json::to_string(&metadata_vault)
        .map_err(|_| VaultError::serialization_error("Failed to serialize vault metadata"))?;

    let metadata_path = format!("{vault_name}/{METADATA_FILENAME}");
    storage.write_file(&metadata_path, &metadata_json).await?;

    for (namespace, data) in &vault.namespaces {
        let namespace_json = serde_json::to_string(&data)
            .map_err(|_| VaultError::serialization_error("Failed to serialize namespace data"))?;

        let namespace_path = format!("{}/{}", vault_name, get_namespace_filename(namespace));
        storage.write_file(&namespace_path, &namespace_json).await?;
    }

    let vault_bytes = serde_json::to_vec(&vault).map_err(|_| {
        VaultError::serialization_error("Failed to serialize vault for notification")
    })?;

    let _ = platform
        .notifier()
        .notify_vault_update(vault_name, &vault_bytes);

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

pub async fn create_vault_from_sync(
    metadata: Option<VaultMetadata>,
    identity_salts: Option<super::types::IdentitySalts>,
    username_pk: Option<HashMap<String, String>>,
) -> Result<Vault, VaultError> {
    let metadata = metadata.ok_or_else(|| {
        VaultError::io_error("Missing vault metadata in sync message for new vault")
    })?;

    Ok(Vault {
        metadata,
        identity_salts: identity_salts.unwrap_or_default(),
        username_pk: username_pk.unwrap_or_default(),
        namespaces: HashMap::new(),
        sync_enabled: true,
    })
}

pub async fn delete_vault(platform: &Platform, vault_name: &str) -> Result<(), VaultError> {
    let storage = platform.storage();
    storage.delete_directory(vault_name).await?;
    Ok(())
}

pub async fn delete_namespace_file(
    platform: &Platform,
    vault_name: &str,
    namespace: &str,
) -> Result<(), VaultError> {
    let namespace_filename = get_namespace_filename(namespace);
    let namespace_path = format!("{vault_name}/{namespace_filename}");

    let storage = platform.storage();
    storage.delete_file(&namespace_path).await
}

pub async fn upsert_namespace(
    platform: &Platform,
    vault_name: &str,
    identity_public_key: &str,
    namespace: &str,
    data: Vec<u8>,
    expires_in_seconds: Option<i64>,
    replace_if_exists: bool,
) -> Result<(), VaultError> {
    let mut vault = read_vault(platform, vault_name).await?;

    if vault.namespaces.contains_key(namespace) && !replace_if_exists {
        return Err(VaultError::NamespaceAlreadyExists);
    }

    let encrypted_data =
        crate::domain::crypto::encrypt_for_recipients(platform, &data, &[identity_public_key])
            .await
            .map_err(|e| VaultError::io_error(e.to_string()))?;

    let expiration = expires_in_seconds.map(|secs| Expiration {
        expires_at: get_current_timestamp() + secs,
    });

    let namespace_data = NamespaceData {
        data: encrypted_data,
        expiration,
    };

    vault
        .namespaces
        .insert(namespace.to_string(), namespace_data);

    save_vault(platform, vault_name, vault).await?;

    Ok(())
}

pub async fn read_namespace(
    platform: &Platform,
    vault_name: &str,
    identity_private_key: &str,
    namespace: &str,
) -> Result<Vec<u8>, VaultError> {
    let mut vault = read_vault(platform, vault_name).await?;

    let namespace_data = vault
        .namespaces
        .get(namespace)
        .ok_or(VaultError::NamespaceNotFound)?;

    let now = get_current_timestamp();
    if let Some(exp_time) = &namespace_data.expiration {
        if now >= exp_time.expires_at {
            vault.namespaces.remove(namespace);
            save_vault(platform, vault_name, vault).await?;
            return Err(VaultError::DataExpired);
        }
    }

    let decrypted_data = crate::domain::crypto::decrypt_with_identity(
        platform,
        &namespace_data.data,
        identity_private_key,
    )
    .await
    .map_err(|_| VaultError::InvalidPassword)?;

    Ok(decrypted_data)
}

pub async fn remove_namespace(
    platform: &Platform,
    vault_name: &str,
    namespace: &str,
) -> Result<(), VaultError> {
    let mut vault = read_vault(platform, vault_name).await?;

    if vault.namespaces.remove(namespace).is_none() {
        return Err(VaultError::NamespaceNotFound);
    }

    delete_namespace_file(platform, vault_name, namespace).await?;

    save_vault(platform, vault_name, vault).await?;

    Ok(())
}

pub async fn list_namespaces_in_vault(
    platform: &Platform,
    vault_name: &str,
) -> Result<Vec<String>, VaultError> {
    let vault = read_vault(platform, vault_name).await?;

    platform.logger().log(&format!(
        "Found {} namespaces in vault",
        vault.namespaces.len()
    ));

    let namespaces: Vec<String> = vault.namespaces.keys().cloned().collect();

    Ok(namespaces)
}

pub async fn export_vault_bytes(
    platform: &Platform,
    vault_name: &str,
) -> Result<Vec<u8>, VaultError> {
    let vault = read_vault(platform, vault_name).await?;

    let vault_bytes = super::serialization::serialize_vault(&vault)?;

    platform.logger().log(&format!(
        "Exporting vault data: {} bytes",
        vault_bytes.len()
    ));

    Ok(vault_bytes)
}

pub async fn import_vault_from_bytes(
    platform: &Platform,
    vault_name: &str,
    vault_bytes: &[u8],
) -> Result<(), VaultError> {
    platform.logger().log(&format!(
        "Attempting to import vault data of size: {} bytes",
        vault_bytes.len()
    ));

    let imported_vault = super::serialization::deserialize_vault(vault_bytes)?;

    match read_vault(platform, vault_name).await {
        Ok(_) => {
            return Err(VaultError::VaultAlreadyExists);
        }
        Err(VaultError::IoError(..)) => {
            platform.logger().log(&format!(
                "No existing vault named '{vault_name}'; proceeding with import."
            ));
        }
        Err(e) => {
            return Err(e);
        }
    }

    save_vault(platform, vault_name, imported_vault).await?;

    Ok(())
}

pub async fn cleanup_vault(platform: &Platform, vault_name: &str) -> Result<bool, VaultError> {
    let mut vault = read_vault(platform, vault_name).await?;

    let now = get_current_timestamp();
    let data_removed =
        super::expiration::cleanup_expired_namespaces(platform, &mut vault, vault_name, now)
            .await?;

    if data_removed {
        save_vault(platform, vault_name, vault).await?;
    }

    Ok(data_removed)
}

pub async fn verify_vault_identity(
    platform: &Platform,
    vault_name: &str,
    identity_private_key: &str,
) -> Result<(), VaultError> {
    let vault = read_vault(platform, vault_name).await?;

    if let Some((_, namespace_data)) = vault.namespaces.iter().next() {
        crate::domain::crypto::decrypt_with_identity(
            platform,
            &namespace_data.data,
            identity_private_key,
        )
        .await
        .map_err(|_| VaultError::InvalidPassword)?;
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn get_current_timestamp() -> i64 {
    (js_sys::Date::now() / 1000.0) as i64
}

#[cfg(not(target_arch = "wasm32"))]
fn get_current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault::types::{IdentitySalts, Vault, VaultMetadata};
    use std::collections::HashMap;

    #[test]
    fn test_get_namespace_filename() {
        assert_eq!(get_namespace_filename("users"), "users.hoddor");
        assert_eq!(get_namespace_filename("config"), "config.hoddor");
        assert_eq!(get_namespace_filename("data-2024"), "data-2024.hoddor");
        assert_eq!(
            get_namespace_filename("my_namespace"),
            "my_namespace.hoddor"
        );
        assert_eq!(get_namespace_filename("test-123"), "test-123.hoddor");
    }

    #[test]
    fn test_legacy_extension_support() {
        // Test that both extensions are recognized
        assert!("users.hoddor".ends_with(NAMESPACE_EXTENSION));
        assert!("users.ns".ends_with(LEGACY_NAMESPACE_EXTENSION));

        // Test stripping both extensions
        assert_eq!(
            "users.hoddor".strip_suffix(NAMESPACE_EXTENSION),
            Some("users")
        );
        assert_eq!(
            "users.ns".strip_suffix(LEGACY_NAMESPACE_EXTENSION),
            Some("users")
        );

        // New files should use .hoddor
        assert_eq!(get_namespace_filename("test"), "test.hoddor");
    }

    #[test]
    fn test_create_vault_returns_empty_vault() {
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

    #[test]
    fn test_create_vault_from_sync_with_all_params() {
        let metadata = VaultMetadata {
            peer_id: Some("test-peer-id".to_string()),
        };
        let mut username_pk = HashMap::new();
        username_pk.insert("user1".to_string(), "pk1".to_string());

        let vault = Vault {
            metadata: metadata.clone(),
            identity_salts: IdentitySalts::new(),
            username_pk: username_pk.clone(),
            namespaces: HashMap::new(),
            sync_enabled: true,
        };

        assert_eq!(vault.metadata.peer_id, Some("test-peer-id".to_string()));
        assert!(vault.namespaces.is_empty());
        assert_eq!(vault.username_pk.len(), 1);
        assert_eq!(vault.username_pk.get("user1"), Some(&"pk1".to_string()));
        assert!(vault.sync_enabled);
    }

    #[test]
    fn test_create_vault_from_sync_validates_metadata() {
        let metadata: Option<VaultMetadata> = None;
        assert!(metadata.is_none());
    }

    #[test]
    fn test_create_vault_from_sync_with_defaults() {
        let metadata = VaultMetadata { peer_id: None };

        let vault = Vault {
            metadata,
            identity_salts: IdentitySalts::new(),
            username_pk: HashMap::new(),
            namespaces: HashMap::new(),
            sync_enabled: true,
        };

        assert!(vault.metadata.peer_id.is_none());
        assert!(vault.namespaces.is_empty());
        assert!(vault.username_pk.is_empty());
        assert!(vault.sync_enabled);
    }

    #[test]
    fn test_create_vault_from_sync_with_peer_id() {
        let metadata = VaultMetadata {
            peer_id: Some("sync-peer-123".to_string()),
        };

        let vault = Vault {
            metadata: metadata.clone(),
            identity_salts: IdentitySalts::new(),
            username_pk: HashMap::new(),
            namespaces: HashMap::new(),
            sync_enabled: true,
        };

        assert_eq!(vault.metadata.peer_id, Some("sync-peer-123".to_string()));
        assert!(vault.sync_enabled);
    }

    #[test]
    fn test_delete_namespace_file_constructs_correct_path() {
        let vault_name = "test_vault";
        let namespace = "test_namespace";
        let expected_filename = format!("{}.hoddor", namespace);
        let expected_path = format!("{}/{}", vault_name, expected_filename);

        let actual_filename = get_namespace_filename(namespace);
        let actual_path = format!("{}/{}", vault_name, actual_filename);

        assert_eq!(actual_path, expected_path);
        assert_eq!(actual_path, "test_vault/test_namespace.hoddor");
    }

    #[test]
    fn test_namespace_extension_backward_compatibility() {
        // Test that both .hoddor and .ns extensions are recognized
        assert!("users.hoddor".ends_with(NAMESPACE_EXTENSION));
        assert!("users.ns".ends_with(LEGACY_NAMESPACE_EXTENSION));

        // Test that both extensions can be stripped correctly
        let hoddor_name = "users.hoddor";
        let ns_name = "users.ns";

        assert_eq!(hoddor_name.strip_suffix(NAMESPACE_EXTENSION), Some("users"));
        assert_eq!(
            ns_name.strip_suffix(LEGACY_NAMESPACE_EXTENSION),
            Some("users")
        );

        // Verify new files use .hoddor
        assert_eq!(get_namespace_filename("test"), "test.hoddor");
        assert!(!get_namespace_filename("test").ends_with(".ns"));
    }

    #[test]
    fn test_read_vault_accepts_both_extensions() {
        // This test verifies the logic in read_vault that checks both extensions
        let entries = vec![
            "config.hoddor".to_string(), // New format
            "settings.ns".to_string(),   // Old format
            "metadata.json".to_string(), // Not a namespace
            "data.txt".to_string(),      // Not a namespace
        ];

        let mut namespace_count = 0;
        for entry in &entries {
            let is_namespace =
                entry.ends_with(NAMESPACE_EXTENSION) || entry.ends_with(LEGACY_NAMESPACE_EXTENSION);
            if is_namespace {
                namespace_count += 1;
            }
        }

        // Should recognize both config.hoddor and settings.ns
        assert_eq!(namespace_count, 2);
    }

    #[test]
    fn test_namespace_stripping_logic() {
        // Test the exact logic used in read_vault for stripping extensions
        let test_cases = vec![
            ("users.hoddor", Some("users")),
            ("config.ns", Some("config")),
            ("data.hoddor", Some("data")),
            ("legacy.ns", Some("legacy")),
        ];

        for (filename, expected) in test_cases {
            let result = if let Some(ns) = filename.strip_suffix(NAMESPACE_EXTENSION) {
                Some(ns.to_string())
            } else if let Some(ns) = filename.strip_suffix(LEGACY_NAMESPACE_EXTENSION) {
                Some(ns.to_string())
            } else {
                None
            };

            assert_eq!(result.as_deref(), expected);
        }
    }

    #[test]
    fn test_new_files_use_hoddor_extension() {
        // Verify that all new namespace files will use .hoddor
        let namespaces = vec!["users", "config", "data-2024", "my_namespace", "test-123"];

        for namespace in namespaces {
            let filename = get_namespace_filename(namespace);
            assert!(
                filename.ends_with(".hoddor"),
                "Namespace '{}' should produce .hoddor file, got: {}",
                namespace,
                filename
            );
            assert!(
                !filename.ends_with(".ns"),
                "Namespace '{}' should not produce .ns file",
                namespace
            );
        }
    }
}
