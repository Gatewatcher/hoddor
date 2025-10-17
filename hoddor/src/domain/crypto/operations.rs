use super::error::CryptoError;
use crate::platform::Platform;

/// Derive an identity from a passphrase using Argon2 + Age
pub async fn identity_from_passphrase(
    platform: &Platform,
    passphrase: &str,
    salt: &[u8],
) -> Result<String, CryptoError> {
    let seed = platform
        .kdf()
        .derive_from_passphrase(passphrase, salt)
        .await
        .map_err(|e| CryptoError::KeyDerivationError(e.to_string()))?;

    platform
        .identity()
        .from_seed(seed)
        .map_err(|e| CryptoError::InvalidIdentity(e.to_string()))
}

/// Generate a new random identity
pub fn generate_identity(platform: &Platform) -> Result<String, CryptoError> {
    platform
        .identity()
        .generate()
        .map_err(|e| CryptoError::InvalidIdentity(e.to_string()))
}

/// Parse a recipient public key
pub fn parse_recipient(platform: &Platform, recipient: &str) -> Result<String, CryptoError> {
    platform
        .identity()
        .parse_recipient(recipient)
        .map_err(|e| CryptoError::InvalidRecipient(e.to_string()))
}

/// Get public key from an identity
pub fn identity_to_public(platform: &Platform, identity: &str) -> Result<String, CryptoError> {
    platform
        .identity()
        .to_public(identity)
        .map_err(|e| CryptoError::InvalidIdentity(e.to_string()))
}

/// Encrypt data for multiple recipients
pub async fn encrypt_for_recipients(
    platform: &Platform,
    data: &[u8],
    recipients: &[&str],
) -> Result<Vec<u8>, CryptoError> {
    platform
        .encryption()
        .encrypt(data, recipients)
        .await
        .map_err(|e| CryptoError::EncryptionError(e.to_string()))
}

/// Decrypt data with an identity
pub async fn decrypt_with_identity(
    platform: &Platform,
    encrypted_data: &[u8],
    identity: &str,
) -> Result<Vec<u8>, CryptoError> {
    platform
        .encryption()
        .decrypt(encrypted_data, identity)
        .await
        .map_err(|e| CryptoError::DecryptionError(e.to_string()))
}

/// Derive an identity from WebAuthn PRF outputs
pub fn identity_from_prf(
    platform: &Platform,
    first: &[u8],
    second: &[u8],
) -> Result<String, CryptoError> {
    if !platform.prf().is_available() {
        return Err(CryptoError::InvalidPrfOutput(
            "PRF not available on this platform".to_string(),
        ));
    }

    let seed = platform
        .prf()
        .derive_from_prf(first, second)
        .map_err(|e| CryptoError::InvalidPrfOutput(e.to_string()))?;

    // Validate seed
    if seed.iter().all(|&x| x == 0) {
        return Err(CryptoError::InvalidPrfOutput(
            "Invalid PRF seed (all zeros)".to_string(),
        ));
    }

    platform
        .identity()
        .from_seed(seed)
        .map_err(|e| CryptoError::InvalidIdentity(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn test_identity_from_passphrase() {
        let platform = Platform::new();
        let result = block_on(identity_from_passphrase(
            &platform,
            "test",
            b"test_salt_16byte",
        ));
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_identity() {
        let platform = Platform::new();
        let result = generate_identity(&platform);
        assert!(result.is_ok());
    }

    #[test]
    fn test_identity_to_public() {
        let platform = Platform::new();
        let identity = generate_identity(&platform).unwrap();
        let public = identity_to_public(&platform, &identity).unwrap();
        assert!(!public.is_empty());
        assert_ne!(identity, public);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let platform = Platform::new();
        let identity = generate_identity(&platform).unwrap();
        let public = identity_to_public(&platform, &identity).unwrap();

        let data = b"secret message";
        let encrypted = block_on(encrypt_for_recipients(&platform, data, &[&public])).unwrap();
        let decrypted = block_on(decrypt_with_identity(&platform, &encrypted, &identity)).unwrap();

        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypt_no_recipients() {
        let platform = Platform::new();
        let data = b"secret";
        let result = block_on(encrypt_for_recipients(&platform, data, &[]));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_recipient() {
        let platform = Platform::new();
        let identity = generate_identity(&platform).unwrap();
        let public = identity_to_public(&platform, &identity).unwrap();

        let parsed = parse_recipient(&platform, &public).unwrap();
        assert_eq!(parsed, public);
    }

    #[test]
    fn test_parse_invalid_recipient() {
        let platform = Platform::new();
        let result = parse_recipient(&platform, "invalid");
        assert!(result.is_err());
    }
}
