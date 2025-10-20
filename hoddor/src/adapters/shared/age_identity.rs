use crate::ports::IdentityPort;
use age::secrecy::ExposeSecret;
use age::x25519::{Identity, Recipient};
use bech32::{ToBase32, Variant};
use std::error::Error;
use x25519_dalek::StaticSecret;
use zeroize::Zeroize;

#[derive(Clone, Copy, Debug)]
pub struct AgeIdentity;

impl AgeIdentity {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgeIdentity {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentityPort for AgeIdentity {
    fn generate(&self) -> Result<String, Box<dyn Error>> {
        let identity = Identity::generate();
        Ok(identity.to_string().expose_secret().to_string())
    }

    fn from_seed(&self, seed: [u8; 32]) -> Result<String, Box<dyn Error>> {
        let mut seed_copy = seed;

        let secret = StaticSecret::from(seed_copy);
        let mut sk_bytes = secret.to_bytes();

        if sk_bytes.iter().all(|&x| x == 0) {
            sk_bytes.zeroize();
            seed_copy.zeroize();
            return Err("Generated invalid secret key (all zeros)".into());
        }

        let sk_base32 = sk_bytes.to_base32();
        let encoded = bech32::encode("age-secret-key-", sk_base32, Variant::Bech32)
            .map_err(|e| {
                sk_bytes.zeroize();
                seed_copy.zeroize();
                format!("Failed to encode identity: {e}")
            })?
            .to_uppercase();

        sk_bytes.zeroize();
        seed_copy.zeroize();

        let identity: Identity = encoded
            .parse()
            .map_err(|e| format!("Failed to parse identity: {e}"))?;

        Ok(identity.to_string().expose_secret().to_string())
    }

    fn parse_recipient(&self, recipient_str: &str) -> Result<String, Box<dyn Error>> {
        let recipient: Recipient = recipient_str
            .parse()
            .map_err(|e| format!("Invalid recipient: {e}"))?;
        Ok(recipient.to_string())
    }

    fn to_public(&self, identity_str: &str) -> Result<String, Box<dyn Error>> {
        let identity: Identity = identity_str
            .parse()
            .map_err(|e| format!("Invalid identity: {e}"))?;
        Ok(identity.to_public().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let adapter = AgeIdentity::new();
        let identity_str = adapter.generate().unwrap();
        assert!(!identity_str.is_empty());
        assert!(identity_str.starts_with("AGE-SECRET-KEY-"));
    }

    #[test]
    fn test_from_seed_deterministic() {
        let adapter = AgeIdentity::new();
        let seed = [42u8; 32];

        let identity1 = adapter.from_seed(seed).unwrap();
        let identity2 = adapter.from_seed(seed).unwrap();

        assert_eq!(identity1, identity2);
    }

    #[test]
    fn test_from_seed_all_zeros_fails() {
        let adapter = AgeIdentity::new();
        let seed = [0u8; 32];

        let result = adapter.from_seed(seed);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_public() {
        let adapter = AgeIdentity::new();
        let identity = adapter.generate().unwrap();
        let public = adapter.to_public(&identity).unwrap();

        assert!(!public.is_empty());
        assert_ne!(identity, public);
        assert!(public.starts_with("age1"));
    }

    #[test]
    fn test_parse_recipient_valid() {
        let adapter = AgeIdentity::new();
        let identity = adapter.generate().unwrap();
        let public = adapter.to_public(&identity).unwrap();

        let parsed = adapter.parse_recipient(&public).unwrap();
        assert_eq!(parsed, public);
    }

    #[test]
    fn test_parse_recipient_invalid() {
        let adapter = AgeIdentity::new();
        let result = adapter.parse_recipient("invalid-recipient");
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_identity_to_public() {
        let adapter = AgeIdentity::new();
        let seed = [123u8; 32];

        let identity = adapter.from_seed(seed).unwrap();
        let public1 = adapter.to_public(&identity).unwrap();
        let public2 = adapter.to_public(&identity).unwrap();

        assert_eq!(public1, public2);
    }
}
