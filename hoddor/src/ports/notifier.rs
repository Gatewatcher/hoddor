pub trait NotifierPort: Send + Sync {
    fn notify_vault_update(&self, vault_name: &str, vault_data: &[u8]) -> Result<(), String>;
}
