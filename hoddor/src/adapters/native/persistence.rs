use crate::domain::vault::error::VaultError;
use crate::ports::PersistencePort;
use async_trait::async_trait;

/// Native persistence stub.
///
/// In native environments, storage is always persistent by default.
/// This adapter always returns true for all operations.
#[derive(Clone, Copy)]
pub struct Persistence;

impl Persistence {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait(?Send)]
impl PersistencePort for Persistence {
    fn has_requested(&self) -> bool {
        true
    }

    async fn request(&self) -> Result<bool, VaultError> {
        Ok(true)
    }

    async fn check(&self) -> Result<bool, VaultError> {
        Ok(true)
    }
}
