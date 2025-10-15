use async_trait::async_trait;
use crate::errors::VaultError;
use crate::ports::PersistencePort;

/// Native persistence stub.
///
/// In native environments, storage is always persistent by default.
/// This adapter always returns true for all operations.
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn test_persistence_creation() {
        let persistence = Persistence::new();
        assert!(persistence.has_requested());
    }

    #[test]
    fn test_persistence_check_always_true() {
        let persistence = Persistence::new();
        let result = block_on(persistence.check());
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_persistence_request_always_true() {
        let persistence = Persistence::new();
        let result = block_on(persistence.request());
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
