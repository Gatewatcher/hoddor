/// Native Rust facade for vault operations
/// Provides an ergonomic Rust API that delegates to domain logic
use crate::domain::authentication;
use crate::domain::vault::{error::VaultError, operations, validation, Vault};
use crate::platform::Platform;

/// Native Rust vault manager
/// Provides high-level vault operations with pure Rust types
pub struct VaultManager {
    platform: Platform,
}

impl VaultManager {
    /// Create a new VaultManager
    pub fn new() -> Self {
        Self {
            platform: Platform::new(),
        }
    }

    /// Derive an identity from a passphrase for a specific vault
    ///
    /// Returns the public and private keys as strings
    pub async fn derive_identity_from_passphrase(
        &self,
        passphrase: &str,
        vault_name: &str,
    ) -> Result<(String, String), VaultError> {
        // Validate inputs
        validation::validate_passphrase(passphrase)?;
        validation::validate_vault_name(vault_name)?;

        // Read vault
        let mut vault = operations::read_vault(&self.platform, vault_name).await?;

        // Derive identity
        let identity_keys = authentication::derive_vault_identity(
            &self.platform,
            passphrase,
            vault_name,
            &mut vault,
        )
        .await
        .map_err(|e| VaultError::io_error(e.to_string()))?;

        // Save vault with new salt if created
        operations::save_vault(&self.platform, vault_name, vault).await?;

        Ok((identity_keys.public_key, identity_keys.private_key))
    }

    /// Insert or update data in a vault namespace
    pub async fn upsert_namespace(
        &self,
        vault_name: &str,
        identity_public_key: &str,
        namespace: &str,
        data: Vec<u8>,
        expires_in_seconds: Option<i64>,
        replace_if_exists: bool,
    ) -> Result<(), VaultError> {
        // Validate namespace
        validation::validate_namespace(namespace)?;

        // Call domain logic
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

    /// Read and decrypt data from a vault namespace
    pub async fn read_namespace(
        &self,
        vault_name: &str,
        identity_private_key: &str,
        namespace: &str,
    ) -> Result<Vec<u8>, VaultError> {
        // Validate namespace
        validation::validate_namespace(namespace)?;

        // Call domain logic
        operations::read_namespace(
            &self.platform,
            vault_name,
            identity_private_key,
            namespace,
        )
        .await
    }

    /// Remove a namespace from a vault
    pub async fn remove_namespace(
        &self,
        vault_name: &str,
        namespace: &str,
    ) -> Result<(), VaultError> {
        // Validate namespace
        validation::validate_namespace(namespace)?;

        // Call domain logic
        operations::remove_namespace(&self.platform, vault_name, namespace).await
    }

    /// List all namespaces in a vault
    pub async fn list_namespaces(&self, vault_name: &str) -> Result<Vec<String>, VaultError> {
        operations::list_namespaces_in_vault(&self.platform, vault_name).await
    }

    /// Create a new vault
    pub async fn create_vault(&self, vault_name: &str) -> Result<(), VaultError> {
        // Validate vault name
        validation::validate_vault_name(vault_name)?;

        // Check if vault already exists
        if operations::read_vault(&self.platform, vault_name)
            .await
            .is_ok()
        {
            return Err(VaultError::VaultAlreadyExists);
        }

        // Create vault
        let vault = operations::create_vault().await?;

        // Save vault
        operations::save_vault(&self.platform, vault_name, vault).await
    }

    /// Remove a vault
    pub async fn remove_vault(&self, vault_name: &str) -> Result<(), VaultError> {
        operations::delete_vault(&self.platform, vault_name).await
    }

    /// List all vaults
    pub async fn list_vaults(&self) -> Result<Vec<String>, VaultError> {
        operations::list_vaults(&self.platform).await
    }

    /// Export a vault as bytes
    pub async fn export_vault(&self, vault_name: &str) -> Result<Vec<u8>, VaultError> {
        operations::export_vault_bytes(&self.platform, vault_name).await
    }

    /// Import a vault from bytes
    pub async fn import_vault(
        &self,
        vault_name: &str,
        vault_bytes: &[u8],
    ) -> Result<(), VaultError> {
        operations::import_vault_from_bytes(&self.platform, vault_name, vault_bytes).await
    }

    /// Force cleanup of expired data in a vault
    pub async fn cleanup_vault(&self, vault_name: &str) -> Result<(), VaultError> {
        // Run cleanup until no more expired data
        loop {
            let data_removed = operations::cleanup_vault(&self.platform, vault_name).await?;
            if !data_removed {
                break;
            }
        }
        Ok(())
    }

    /// Verify that an identity can decrypt a vault
    pub async fn verify_identity(
        &self,
        vault_name: &str,
        identity_private_key: &str,
    ) -> Result<(), VaultError> {
        operations::verify_vault_identity(&self.platform, vault_name, identity_private_key).await
    }

    /// Read a vault directly (low-level API)
    pub async fn read_vault(&self, vault_name: &str) -> Result<Vault, VaultError> {
        operations::read_vault(&self.platform, vault_name).await
    }

    /// Save a vault directly (low-level API)
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
        // Just verify it compiles and can be created
        assert!(std::mem::size_of_val(&manager) > 0);
    }

    #[test]
    fn test_vault_manager_default() {
        let manager = VaultManager::default();
        assert!(std::mem::size_of_val(&manager) > 0);
    }
}
