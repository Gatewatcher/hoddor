use crate::ports::PrfPort;
use std::error::Error;

/// Mock PRF adapter - PRF not available in native builds
///
/// In the future, this could be replaced with a MFA framework.
#[derive(Clone, Copy, Debug)]
pub struct MockPrf;

impl MockPrf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockPrf {
    fn default() -> Self {
        Self::new()
    }
}

impl PrfPort for MockPrf {
    fn derive_from_prf(&self, _first: &[u8], _second: &[u8]) -> Result<[u8; 32], Box<dyn Error>> {
        Err("PRF (WebAuthn) not available in native builds".into())
    }

    fn is_available(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prf_not_available() {
        let adapter = MockPrf::new();
        assert!(!adapter.is_available());
        assert!(adapter.derive_from_prf(&[1u8; 32], &[2u8; 32]).is_err());
    }

    #[test]
    fn test_prf_error_message() {
        let adapter = MockPrf::new();
        let result = adapter.derive_from_prf(&[1u8; 32], &[2u8; 32]);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not available in native"));
    }
}
