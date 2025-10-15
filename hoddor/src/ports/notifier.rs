/// Notifier port - provides event notification capabilities across platforms.
///
/// Abstracts notifications from platform-specific implementations:
/// - WASM: postMessage API for inter-context communication (window/worker)
/// - Native: No-op (single process, no inter-context communication needed)
pub trait NotifierPort: Send + Sync {
    /// Notify that a vault has been updated.
    ///
    /// On WASM, broadcasts the update to other browser contexts (tabs, workers).
    /// On native, this is a no-op.
    fn notify_vault_update(&self, vault_name: &str, vault_data: &[u8]) -> Result<(), String>;
}
