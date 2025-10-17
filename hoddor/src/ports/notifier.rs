/// Notifier port - provides event notification capabilities across platforms.
///
/// Abstracts notifications from platform-specific implementations:
/// - WASM: postMessage API for inter-context communication (window/worker)
/// - Native: No-op (single process, no inter-context communication needed)
pub trait NotifierPort: Send + Sync {
    fn notify_vault_update(&self, vault_name: &str, vault_data: &[u8]) -> Result<(), String>;
}
