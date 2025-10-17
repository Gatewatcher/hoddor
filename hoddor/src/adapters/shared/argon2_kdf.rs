use crate::ports::KeyDerivationPort;
use argon2::Argon2;
use async_trait::async_trait;
use std::error::Error;

#[derive(Clone, Copy, Debug)]
pub struct Argon2Kdf;

impl Argon2Kdf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Argon2Kdf {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl KeyDerivationPort for Argon2Kdf {
    async fn derive_from_passphrase(
        &self,
        passphrase: &str,
        salt: &[u8],
    ) -> Result<[u8; 32], Box<dyn Error>> {
        if passphrase.is_empty() || passphrase.trim().is_empty() {
            return Err("Passphrase cannot be empty or whitespace-only".into());
        }

        let argon2 = Argon2::default();
        let mut seed = [0u8; 32];
        argon2
            .hash_password_into(passphrase.as_bytes(), salt, &mut seed)
            .map_err(|e| format!("Argon2 derivation failed: {:?}", e))?;
        Ok(seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn test_derive_is_deterministic() {
        let adapter = Argon2Kdf::new();
        let passphrase = "test password";
        let salt = b"test_salt_16byte";

        let seed1 = block_on(adapter.derive_from_passphrase(passphrase, salt)).unwrap();
        let seed2 = block_on(adapter.derive_from_passphrase(passphrase, salt)).unwrap();

        assert_eq!(seed1, seed2);
    }

    #[test]
    fn test_different_passwords_different_seeds() {
        let adapter = Argon2Kdf::new();
        let salt = b"test_salt_16byte";

        let seed1 = block_on(adapter.derive_from_passphrase("password1", salt)).unwrap();
        let seed2 = block_on(adapter.derive_from_passphrase("password2", salt)).unwrap();

        assert_ne!(seed1, seed2);
    }

    #[test]
    fn test_different_salts_different_seeds() {
        let adapter = Argon2Kdf::new();
        let passphrase = "test password";

        let seed1 =
            block_on(adapter.derive_from_passphrase(passphrase, b"salt1_test_16byt")).unwrap();
        let seed2 =
            block_on(adapter.derive_from_passphrase(passphrase, b"salt2_test_16byt")).unwrap();

        assert_ne!(seed1, seed2);
    }

    #[test]
    fn test_empty_passphrase() {
        let adapter = Argon2Kdf::new();
        let salt = b"test_salt_16byte";

        let result = block_on(adapter.derive_from_passphrase("", salt));
        assert!(result.is_err(), "Empty passphrase should be rejected");
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_whitespace_only_passphrase() {
        let adapter = Argon2Kdf::new();
        let salt = b"test_salt_16byte";

        let result = block_on(adapter.derive_from_passphrase("   ", salt));
        assert!(result.is_err(), "Whitespace-only passphrase should be rejected");
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_output_length() {
        let adapter = Argon2Kdf::new();
        let passphrase = "test";
        let salt = b"test_salt_16byte";

        let seed = block_on(adapter.derive_from_passphrase(passphrase, salt)).unwrap();
        assert_eq!(seed.len(), 32);
    }
}
