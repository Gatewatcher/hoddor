use super::error::AuthenticationError;
use super::types::IdentityKeys;
use crate::domain::vault::types::Vault;
use crate::domain::vault::validation::validate_passphrase;
use crate::platform::Platform;
use argon2::password_hash::rand_core::OsRng;
use rand::RngCore;

/// Derives an identity from a passphrase for a specific vault
///
/// This function first searches for an existing identity in the vault.
/// If no identity matches the passphrase, it creates a new one.
pub async fn derive_vault_identity(
    platform: &Platform,
    passphrase: &str,
    _vault_name: &str,
    vault: &mut Vault,
) -> Result<IdentityKeys, AuthenticationError> {
    validate_passphrase(passphrase)
        .map_err(|e| AuthenticationError::InvalidPassphrase(e.to_string()))?;

    for (stored_pubkey, salt) in vault.identity_salts.iter() {
        platform
            .logger()
            .log(&format!("Checking stored public key: {}", stored_pubkey));

        if salt.len() != 32 {
            platform.logger().error(&format!(
                "Invalid salt length ({}) for public key: {}",
                salt.len(),
                stored_pubkey
            ));
            continue;
        }

        platform.logger().log(&format!("Using salt: {:?}", salt));

        match derive_identity_from_passphrase(platform, passphrase, salt).await {
            Ok(identity) => {
                platform
                    .logger()
                    .log(&format!("Generated public key: {}", identity.public_key));
                if identity.public_key == *stored_pubkey {
                    platform.logger().log("Found matching identity");
                    return Ok(identity);
                } else {
                    platform
                        .logger()
                        .warn("Public key does not match stored salt");
                }
            }
            Err(err) => {
                platform.logger().warn(&format!(
                    "Failed to generate identity with stored salt for public key {}: {:?}",
                    stored_pubkey, err
                ));
            }
        }
    }

    platform
        .logger()
        .log("No matching identity found; generating new salt");
    let mut new_salt = [0u8; 32];
    OsRng.fill_bytes(&mut new_salt);

    let identity = derive_identity_from_passphrase(platform, passphrase, &new_salt)
        .await
        .map_err(|e| {
            platform
                .logger()
                .error(&format!("Failed to create new identity: {:?}", e));
            e
        })?;

    vault
        .identity_salts
        .set_salt(identity.public_key.clone(), new_salt);

    Ok(identity)
}

/// Derives an identity from a passphrase and salt
///
/// Internal function that performs cryptographic derivation.
async fn derive_identity_from_passphrase(
    platform: &Platform,
    passphrase: &str,
    salt: &[u8],
) -> Result<IdentityKeys, AuthenticationError> {
    if salt.len() != 32 {
        return Err(AuthenticationError::InvalidSalt(format!(
            "Salt must be 32 bytes, got {}",
            salt.len()
        )));
    }

    let identity_str = crate::domain::crypto::identity_from_passphrase(platform, passphrase, salt)
        .await
        .map_err(|e| {
            platform
                .logger()
                .log(&format!("Failed to derive identity: {}", e));
            AuthenticationError::DerivationFailed(e.to_string())
        })?;

    let identity: age::x25519::Identity = identity_str
        .parse()
        .map_err(|e| AuthenticationError::InvalidIdentityFormat(format!("{}", e)))?;

    let public_key = identity.to_public().to_string();
    let private_key = {
        use age::secrecy::ExposeSecret;
        identity.to_string().expose_secret().to_string()
    };

    Ok(IdentityKeys::new(public_key, private_key))
}

/// Generates a new random identity
pub fn generate_random_identity(platform: &Platform) -> Result<IdentityKeys, AuthenticationError> {
    let identity_str = crate::domain::crypto::generate_identity(platform)
        .map_err(|e| AuthenticationError::RandomGenerationFailed(e.to_string()))?;

    let identity: age::x25519::Identity = identity_str
        .parse()
        .map_err(|e| AuthenticationError::InvalidIdentityFormat(format!("{}", e)))?;

    let public_key = identity.to_public().to_string();
    let private_key = {
        use age::secrecy::ExposeSecret;
        identity.to_string().expose_secret().to_string()
    };

    Ok(IdentityKeys::new(public_key, private_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_keys_creation() {
        let keys = IdentityKeys::new(
            "age1test123".to_string(),
            "AGE-SECRET-KEY-1TEST".to_string(),
        );
        assert_eq!(keys.public_key, "age1test123");
        assert_eq!(keys.private_key, "AGE-SECRET-KEY-1TEST");
    }
}
