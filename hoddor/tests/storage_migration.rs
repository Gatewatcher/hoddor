use hoddor::{read_vault_with_name, save_vault, Vault, VaultMetadata, IdentitySalts};
use std::collections::HashMap;
use futures::executor::block_on;

#[test]
fn test_save_and_read_vault_with_storage_port() {
    block_on(async {
        let vault_name = "test_vault_storage";

        let vault = Vault {
            metadata: VaultMetadata { peer_id: None },
            identity_salts: IdentitySalts::new(),
            username_pk: HashMap::new(),
            namespaces: HashMap::new(),
            sync_enabled: false,
        };

        save_vault(vault_name, vault.clone()).await.expect("Failed to save vault");

        let loaded_vault = read_vault_with_name(vault_name).await.expect("Failed to read vault");

        assert_eq!(loaded_vault.metadata.peer_id, vault.metadata.peer_id);
        assert_eq!(loaded_vault.sync_enabled, vault.sync_enabled);
        assert_eq!(loaded_vault.namespaces.len(), 0);

        let platform = hoddor::Platform::new();
        let storage = platform.storage();
        storage.delete_directory(vault_name).await.ok();
    });
}

#[test]
fn test_read_nonexistent_vault() {
    block_on(async {
        let vault_name = "nonexistent_vault_test";

        let result = read_vault_with_name(vault_name).await;

        assert!(result.is_err());
    });
}
