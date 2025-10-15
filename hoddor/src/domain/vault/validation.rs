use crate::errors::VaultError;

pub fn validate_namespace(namespace: &str) -> Result<(), VaultError> {
    if namespace.trim().is_empty() {
        return Err(VaultError::IoError {
            message: "Namespace cannot be empty or whitespace only",
        });
    }

    let invalid_chars = ['/', '\\', '<', '>', ':', '"', '|', '?', '*'];
    if namespace.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(VaultError::IoError {
            message: "Namespace contains invalid characters",
        });
    }
    Ok(())
}

pub fn validate_passphrase(passphrase: &str) -> Result<(), VaultError> {
    if passphrase.trim().is_empty() {
        return Err(VaultError::JsError(
            "Passphrase cannot be empty or whitespace".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_vault_name(name: &str) -> Result<(), VaultError> {
    if name.trim().is_empty() {
        return Err(VaultError::IoError {
            message: "Vault name cannot be empty or whitespace only",
        });
    }
    if name.contains(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-') {
        return Err(VaultError::IoError {
            message:
                "Vault name can only contain alphanumeric characters, underscores, and hyphens",
        });
    }
    Ok(())
}
