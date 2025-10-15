use crate::errors::VaultError;
use crate::platform::Platform;
use super::operations::{get_namespace_filename, get_vault_dirname};
use super::types::{Expiration, Vault};

pub fn is_expired(expiration: &Option<Expiration>, now: i64) -> bool {
    if let Some(exp) = expiration {
        now >= exp.expires_at
    } else {
        false
    }
}

pub fn create_expiration(expires_in_seconds: Option<i64>, now: i64) -> Option<Expiration> {
    if let Some(seconds) = expires_in_seconds {
        if seconds <= 0 {
            None
        } else {
            Some(Expiration {
                expires_at: now + seconds,
            })
        }
    } else {
        None
    }
}

pub async fn cleanup_expired_namespaces(
    platform: &Platform,
    vault: &mut Vault,
    vault_name: &str,
    now: i64,
) -> Result<bool, VaultError> {
    let mut data_removed = false;

    let expired_namespaces: Vec<String> = vault
        .namespaces
        .iter()
        .filter_map(|(namespace, encrypted)| {
            if is_expired(&encrypted.expiration, now) {
                Some(namespace.clone())
            } else {
                None
            }
        })
        .collect();

    let dirname = get_vault_dirname(vault_name);
    let storage = platform.storage();

    for namespace in expired_namespaces {
        let namespace_filename = get_namespace_filename(&namespace);
        let namespace_path = format!("{}/{}", dirname, namespace_filename);
        let _ = storage.delete_file(&namespace_path).await;
        vault.namespaces.remove(&namespace);
        data_removed = true;
        platform.logger().log(&format!("Removed expired namespace: {}", namespace));
    }

    Ok(data_removed)
}
