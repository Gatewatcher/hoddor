use crate::ports::NotifierPort;

/// Native notifier adapter (no-op).
///
/// On native, there's no need for inter-context notifications since
/// it's a single process with no workers or multiple tabs.
#[derive(Clone, Copy)]
pub struct Notifier;

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifier {
    pub fn new() -> Self {
        Self
    }
}

impl NotifierPort for Notifier {
    fn notify_vault_update(&self, _vault_name: &str, _vault_data: &[u8]) -> Result<(), String> {
        Ok(())
    }
}
