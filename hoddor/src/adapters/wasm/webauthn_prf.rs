use crate::ports::PrfPort;
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use std::error::Error;

/// WebAuthn PRF adapter - only available in WASM
#[derive(Clone, Copy, Debug)]
pub struct WebAuthnPrf;

impl WebAuthnPrf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebAuthnPrf {
    fn default() -> Self {
        Self::new()
    }
}

impl PrfPort for WebAuthnPrf {
    fn derive_from_prf(&self, first: &[u8], second: &[u8]) -> Result<[u8; 32], Box<dyn Error>> {
        if first.is_empty() {
            return Err("Missing first PRF value".into());
        }
        if second.is_empty() {
            return Err("Missing second PRF value".into());
        }

        let mut prf = first.to_vec();
        prf.extend(second);

        let mixed_prf = Sha256::digest(&prf);
        let (prk, _) =
            Hkdf::<Sha256>::extract(Some("hoddor/vault".as_bytes()), mixed_prf.as_slice());

        Ok(prk.into())
    }

    fn is_available(&self) -> bool {
        true
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_prf_derivation() {
        let adapter = WebAuthnPrf::new();
        let first = vec![1u8; 32];
        let second = vec![2u8; 32];

        let key = adapter.derive_from_prf(&first, &second).unwrap();
        assert_eq!(key.len(), 32);
    }

    #[wasm_bindgen_test]
    fn test_prf_is_available() {
        let adapter = WebAuthnPrf::new();
        assert!(adapter.is_available());
    }

    #[wasm_bindgen_test]
    fn test_prf_deterministic() {
        let adapter = WebAuthnPrf::new();
        let first = vec![42u8; 32];
        let second = vec![84u8; 32];

        let key1 = adapter.derive_from_prf(&first, &second).unwrap();
        let key2 = adapter.derive_from_prf(&first, &second).unwrap();

        assert_eq!(key1, key2);
    }

    #[wasm_bindgen_test]
    fn test_prf_missing_first() {
        let adapter = WebAuthnPrf::new();
        let result = adapter.derive_from_prf(&[], &vec![2u8; 32]);
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_prf_missing_second() {
        let adapter = WebAuthnPrf::new();
        let result = adapter.derive_from_prf(&vec![1u8; 32], &[]);
        assert!(result.is_err());
    }
}
