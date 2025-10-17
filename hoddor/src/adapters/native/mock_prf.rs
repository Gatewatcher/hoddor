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
