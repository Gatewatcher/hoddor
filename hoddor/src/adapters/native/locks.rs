use crate::domain::vault::error::VaultError;
use crate::ports::{LockGuard, LockPort};
use async_trait::async_trait;

pub struct NativeLockGuard;

impl LockGuard for NativeLockGuard {}

#[derive(Clone, Copy)]
pub struct Locks;

impl Default for Locks {
    fn default() -> Self {
        Self::new()
    }
}

impl Locks {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait(?Send)]
impl LockPort for Locks {
    async fn acquire(&self, _name: &str) -> Result<Box<dyn LockGuard>, VaultError> {
        Ok(Box::new(NativeLockGuard))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn test_locks_creation() {
        let _locks = Locks::new();
    }

    #[test]
    fn test_acquire_lock_succeeds() {
        let locks = Locks::new();
        let result = block_on(locks.acquire("test_vault"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_guard_drop() {
        let locks = Locks::new();
        let _guard = block_on(locks.acquire("test_vault")).unwrap();
    }
}
