use super::error::VaultError;
use super::operations::get_namespace_filename;
use super::types::{Expiration, Vault};
use crate::platform::Platform;

pub fn is_expired(expiration: &Option<Expiration>, now: i64) -> bool {
    expiration.as_ref().is_some_and(|exp| now >= exp.expires_at)
}

pub fn create_expiration(expires_in_seconds: Option<i64>, now: i64) -> Option<Expiration> {
    expires_in_seconds
        .filter(|&seconds| seconds > 0)
        .map(|seconds| Expiration {
            expires_at: now + seconds,
        })
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

    let storage = platform.storage();

    for namespace in expired_namespaces {
        let namespace_filename = get_namespace_filename(&namespace);
        let namespace_path = format!("{vault_name}/{namespace_filename}");
        let _ = storage.delete_file(&namespace_path).await;
        vault.namespaces.remove(&namespace);
        data_removed = true;
        platform
            .logger()
            .log(&format!("Removed expired namespace: {namespace}"));
    }

    Ok(data_removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_expired_with_no_expiration() {
        let now = 1000;
        assert!(!is_expired(&None, now));
    }

    #[test]
    fn test_is_expired_with_future_expiration() {
        let now = 1000;
        let expiration = Some(Expiration { expires_at: 2000 });
        assert!(!is_expired(&expiration, now));
    }

    #[test]
    fn test_is_expired_with_exact_expiration() {
        let now = 1000;
        let expiration = Some(Expiration { expires_at: 1000 });
        assert!(is_expired(&expiration, now));
    }

    #[test]
    fn test_is_expired_with_past_expiration() {
        let now = 2000;
        let expiration = Some(Expiration { expires_at: 1000 });
        assert!(is_expired(&expiration, now));
    }

    #[test]
    fn test_create_expiration_with_none() {
        let now = 1000;
        let result = create_expiration(None, now);
        assert!(result.is_none());
    }

    #[test]
    fn test_create_expiration_with_zero() {
        let now = 1000;
        let result = create_expiration(Some(0), now);
        assert!(result.is_none());
    }

    #[test]
    fn test_create_expiration_with_negative() {
        let now = 1000;
        let result = create_expiration(Some(-100), now);
        assert!(result.is_none());
    }

    #[test]
    fn test_create_expiration_with_positive() {
        let now = 1000;
        let result = create_expiration(Some(500), now);
        assert!(result.is_some());
        let expiration = result.unwrap();
        assert_eq!(expiration.expires_at, 1500);
    }

    #[test]
    fn test_create_expiration_large_duration() {
        let now = 1000;
        let one_year_seconds = 365 * 24 * 60 * 60;
        let result = create_expiration(Some(one_year_seconds), now);
        assert!(result.is_some());
        let expiration = result.unwrap();
        assert_eq!(expiration.expires_at, now + one_year_seconds);
    }
}
