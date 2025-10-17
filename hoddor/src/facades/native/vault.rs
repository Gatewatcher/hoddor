use crate::domain::authentication;
use crate::domain::vault::{error::VaultError, operations, validation, Vault};
use crate::platform::Platform;

pub struct VaultManager {
    platform: Platform,
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            platform: Platform::new(),
        }
    }

    pub async fn derive_identity_from_passphrase(
        &self,
        passphrase: &str,
        vault_name: &str,
    ) -> Result<(String, String), VaultError> {
        validation::validate_passphrase(passphrase)?;
        validation::validate_vault_name(vault_name)?;

        let mut vault = operations::read_vault(&self.platform, vault_name).await?;

        let identity_keys = authentication::derive_vault_identity(
            &self.platform,
            passphrase,
            vault_name,
            &mut vault,
        )
        .await
        .map_err(|e| VaultError::io_error(e.to_string()))?;

        operations::save_vault(&self.platform, vault_name, vault).await?;

        Ok((identity_keys.public_key, identity_keys.private_key))
    }

    pub async fn upsert_namespace(
        &self,
        vault_name: &str,
        identity_public_key: &str,
        namespace: &str,
        data: Vec<u8>,
        expires_in_seconds: Option<i64>,
        replace_if_exists: bool,
    ) -> Result<(), VaultError> {
        validation::validate_namespace(namespace)?;

        operations::upsert_namespace(
            &self.platform,
            vault_name,
            identity_public_key,
            namespace,
            data,
            expires_in_seconds,
            replace_if_exists,
        )
        .await
    }

    pub async fn read_namespace(
        &self,
        vault_name: &str,
        identity_private_key: &str,
        namespace: &str,
    ) -> Result<Vec<u8>, VaultError> {
        validation::validate_namespace(namespace)?;

        operations::read_namespace(&self.platform, vault_name, identity_private_key, namespace)
            .await
    }

    pub async fn remove_namespace(
        &self,
        vault_name: &str,
        namespace: &str,
    ) -> Result<(), VaultError> {
        validation::validate_namespace(namespace)?;

        operations::remove_namespace(&self.platform, vault_name, namespace).await
    }

    pub async fn list_namespaces(&self, vault_name: &str) -> Result<Vec<String>, VaultError> {
        operations::list_namespaces_in_vault(&self.platform, vault_name).await
    }

    pub async fn create_vault(&self, vault_name: &str) -> Result<(), VaultError> {
        validation::validate_vault_name(vault_name)?;

        if operations::read_vault(&self.platform, vault_name)
            .await
            .is_ok()
        {
            return Err(VaultError::VaultAlreadyExists);
        }

        let vault = operations::create_vault().await?;

        operations::save_vault(&self.platform, vault_name, vault).await
    }

    pub async fn remove_vault(&self, vault_name: &str) -> Result<(), VaultError> {
        operations::delete_vault(&self.platform, vault_name).await
    }

    pub async fn list_vaults(&self) -> Result<Vec<String>, VaultError> {
        operations::list_vaults(&self.platform).await
    }

    pub async fn export_vault(&self, vault_name: &str) -> Result<Vec<u8>, VaultError> {
        operations::export_vault_bytes(&self.platform, vault_name).await
    }

    pub async fn import_vault(
        &self,
        vault_name: &str,
        vault_bytes: &[u8],
    ) -> Result<(), VaultError> {
        operations::import_vault_from_bytes(&self.platform, vault_name, vault_bytes).await
    }

    pub async fn cleanup_vault(&self, vault_name: &str) -> Result<(), VaultError> {
        loop {
            let data_removed = operations::cleanup_vault(&self.platform, vault_name).await?;
            if !data_removed {
                break;
            }
        }
        Ok(())
    }

    pub async fn verify_identity(
        &self,
        vault_name: &str,
        identity_private_key: &str,
    ) -> Result<(), VaultError> {
        operations::verify_vault_identity(&self.platform, vault_name, identity_private_key).await
    }

    pub async fn read_vault(&self, vault_name: &str) -> Result<Vault, VaultError> {
        operations::read_vault(&self.platform, vault_name).await
    }

    pub async fn save_vault(&self, vault_name: &str, vault: Vault) -> Result<(), VaultError> {
        operations::save_vault(&self.platform, vault_name, vault).await
    }
}

impl Default for VaultManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_manager_creation() {
        let manager = VaultManager::new();
        assert!(std::mem::size_of_val(&manager) > 0);
    }

    #[test]
    fn test_vault_manager_default() {
        let manager = VaultManager::default();
        assert!(std::mem::size_of_val(&manager) > 0);
    }
}
