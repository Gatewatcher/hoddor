use super::error::VaultError;
use super::types::Vault;

const VAULT_MAGIC_NUMBER: &[u8; 6] = b"VAULT1";

pub fn serialize_vault(vault: &Vault) -> Result<Vec<u8>, VaultError> {
    let serialized = serde_json::to_vec(vault)
        .map_err(|_| VaultError::serialization_error("Failed to serialize vault for export"))?;

    let total_size = VAULT_MAGIC_NUMBER.len() + 4 + serialized.len();
    let mut vault_bytes = Vec::with_capacity(total_size);

    vault_bytes.extend_from_slice(VAULT_MAGIC_NUMBER);
    vault_bytes.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
    vault_bytes.extend_from_slice(&serialized);

    Ok(vault_bytes)
}

pub fn deserialize_vault(vault_bytes: &[u8]) -> Result<Vault, VaultError> {
    if vault_bytes.len() < 10 || &vault_bytes[..6] != VAULT_MAGIC_NUMBER {
        return Err(VaultError::serialization_error(
            "Invalid vault file: missing or incorrect magic number",
        ));
    }

    let length = u32::from_be_bytes([
        vault_bytes[6],
        vault_bytes[7],
        vault_bytes[8],
        vault_bytes[9],
    ]) as usize;

    if vault_bytes.len() != length + 10 {
        return Err(VaultError::serialization_error(
            "Invalid vault file: content length mismatch",
        ));
    }

    let vault: Vault = serde_json::from_slice(&vault_bytes[10..])
        .map_err(|_| VaultError::serialization_error("Failed to deserialize vault data"))?;

    Ok(vault)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault::types::{IdentitySalts, VaultMetadata};
    use std::collections::HashMap;

    #[test]
    fn test_serialize_vault() {
        let vault = Vault {
            metadata: VaultMetadata { peer_id: None },
            identity_salts: IdentitySalts::new(),
            username_pk: HashMap::new(),
            namespaces: HashMap::new(),
            sync_enabled: false,
        };

        let result = serialize_vault(&vault);
        assert!(result.is_ok());

        let bytes = result.unwrap();
        assert_eq!(&bytes[0..6], b"VAULT1");
        assert!(bytes.len() > 10);
    }

    #[test]
    fn test_deserialize_vault() {
        let vault = Vault {
            metadata: VaultMetadata {
                peer_id: Some("test-peer".to_string()),
            },
            identity_salts: IdentitySalts::new(),
            username_pk: HashMap::new(),
            namespaces: HashMap::new(),
            sync_enabled: true,
        };

        let bytes = serialize_vault(&vault).unwrap();
        let deserialized = deserialize_vault(&bytes).unwrap();

        assert_eq!(deserialized.metadata.peer_id, Some("test-peer".to_string()));
        assert_eq!(deserialized.sync_enabled, true);
    }

    #[test]
    fn test_deserialize_vault_invalid_magic_number() {
        let invalid_bytes = b"INVALID_HEADER_DATA";
        let result = deserialize_vault(invalid_bytes);

        assert!(result.is_err());
        match result {
            Err(VaultError::SerializationError(msg)) => {
                assert!(msg.contains("magic number"));
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_deserialize_vault_too_short() {
        let short_bytes = b"VAULT1";
        let result = deserialize_vault(short_bytes);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_vault_length_mismatch() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VAULT1");
        bytes.extend_from_slice(&100u32.to_be_bytes());
        bytes.extend_from_slice(b"{}");

        let result = deserialize_vault(&bytes);
        assert!(result.is_err());
        match result {
            Err(VaultError::SerializationError(msg)) => {
                assert!(msg.contains("length mismatch"));
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_roundtrip_serialization() {
        let mut username_pk = HashMap::new();
        username_pk.insert("user1".to_string(), "pk1".to_string());
        username_pk.insert("user2".to_string(), "pk2".to_string());

        let vault = Vault {
            metadata: VaultMetadata {
                peer_id: Some("peer-123".to_string()),
            },
            identity_salts: IdentitySalts::new(),
            username_pk,
            namespaces: HashMap::new(),
            sync_enabled: true,
        };

        let serialized = serialize_vault(&vault).unwrap();
        let deserialized = deserialize_vault(&serialized).unwrap();

        assert_eq!(vault.metadata.peer_id, deserialized.metadata.peer_id);
        assert_eq!(vault.sync_enabled, deserialized.sync_enabled);
        assert_eq!(vault.username_pk.len(), deserialized.username_pk.len());
        assert_eq!(
            vault.username_pk.get("user1"),
            deserialized.username_pk.get("user1")
        );
    }
}
