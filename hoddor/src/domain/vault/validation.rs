use super::error::VaultError;

fn validate_not_empty(value: &str, error_msg: &str) -> Result<(), VaultError> {
    if value.trim().is_empty() {
        return Err(VaultError::io_error(error_msg));
    }
    Ok(())
}

pub fn validate_namespace(namespace: &str) -> Result<(), VaultError> {
    validate_not_empty(namespace, "Namespace cannot be empty or whitespace only")?;

    let invalid_chars = ['/', '\\', '<', '>', ':', '"', '|', '?', '*'];
    if namespace.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(VaultError::io_error(
            "Namespace contains invalid characters",
        ));
    }
    Ok(())
}

pub fn validate_passphrase(passphrase: &str) -> Result<(), VaultError> {
    validate_not_empty(passphrase, "Passphrase cannot be empty or whitespace only")
}

pub fn validate_vault_name(name: &str) -> Result<(), VaultError> {
    validate_not_empty(name, "Vault name cannot be empty or whitespace only")?;
    if name.contains(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-') {
        return Err(VaultError::io_error(
            "Vault name can only contain alphanumeric characters, underscores, and hyphens",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for validate_namespace
    #[test]
    fn test_validate_namespace_valid() {
        assert!(validate_namespace("test").is_ok());
        assert!(validate_namespace("my-namespace").is_ok());
        assert!(validate_namespace("namespace_123").is_ok());
        assert!(validate_namespace("CamelCase").is_ok());
    }

    #[test]
    fn test_validate_namespace_empty() {
        assert!(validate_namespace("").is_err());
    }

    #[test]
    fn test_validate_namespace_whitespace_only() {
        assert!(validate_namespace("   ").is_err());
        assert!(validate_namespace("\t").is_err());
        assert!(validate_namespace("\n").is_err());
    }

    #[test]
    fn test_validate_namespace_invalid_characters() {
        assert!(validate_namespace("test/path").is_err());
        assert!(validate_namespace("test\\path").is_err());
        assert!(validate_namespace("test<file").is_err());
        assert!(validate_namespace("test>file").is_err());
        assert!(validate_namespace("test:file").is_err());
        assert!(validate_namespace("test\"file").is_err());
        assert!(validate_namespace("test|file").is_err());
        assert!(validate_namespace("test?file").is_err());
        assert!(validate_namespace("test*file").is_err());
    }

    // Tests for validate_passphrase
    #[test]
    fn test_validate_passphrase_valid() {
        assert!(validate_passphrase("password123").is_ok());
        assert!(validate_passphrase("my secure passphrase").is_ok());
        assert!(validate_passphrase("!@#$%^&*()").is_ok());
    }

    #[test]
    fn test_validate_passphrase_empty() {
        assert!(validate_passphrase("").is_err());
    }

    #[test]
    fn test_validate_passphrase_whitespace_only() {
        assert!(validate_passphrase("   ").is_err());
        assert!(validate_passphrase("\t\t").is_err());
    }

    // Tests for validate_vault_name
    #[test]
    fn test_validate_vault_name_valid() {
        assert!(validate_vault_name("vault1").is_ok());
        assert!(validate_vault_name("my_vault").is_ok());
        assert!(validate_vault_name("my-vault").is_ok());
        assert!(validate_vault_name("vault123").is_ok());
        assert!(validate_vault_name("MyVault").is_ok());
    }

    #[test]
    fn test_validate_vault_name_empty() {
        assert!(validate_vault_name("").is_err());
    }

    #[test]
    fn test_validate_vault_name_whitespace_only() {
        assert!(validate_vault_name("   ").is_err());
    }

    #[test]
    fn test_validate_vault_name_invalid_characters() {
        assert!(validate_vault_name("vault name").is_err()); // space
        assert!(validate_vault_name("vault/name").is_err()); // slash
        assert!(validate_vault_name("vault.name").is_err()); // dot
        assert!(validate_vault_name("vault@name").is_err()); // @
        assert!(validate_vault_name("vault#name").is_err()); // #
    }

    #[test]
    fn test_validate_vault_name_special_allowed() {
        assert!(validate_vault_name("vault_name").is_ok()); // underscore
        assert!(validate_vault_name("vault-name").is_ok()); // hyphen
    }
}
