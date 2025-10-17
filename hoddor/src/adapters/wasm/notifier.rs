use crate::global::get_global_scope;
use crate::notifications;
use crate::ports::NotifierPort;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy)]
pub struct Notifier;

impl Notifier {
    pub fn new() -> Self {
        Self
    }
}

impl NotifierPort for Notifier {
    fn notify_vault_update(&self, _vault_name: &str, vault_data: &[u8]) -> Result<(), String> {
        let global_scope = get_global_scope().map_err(|e| format!("{:?}", e))?;

        let vault: crate::domain::vault::Vault = serde_json::from_slice(vault_data)
            .map_err(|e| format!("Failed to deserialize vault: {}", e))?;

        let msg = notifications::Message {
            event: notifications::EventType::VaultUpdate,
            data: vault,
        };

        let js_value = serde_wasm_bindgen::to_value(&msg)
            .map_err(|e| format!("Failed to serialize: {:?}", e))?;

        if let Ok(worker_scope) = global_scope
            .clone()
            .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
        {
            worker_scope
                .post_message(&js_value)
                .map_err(|e| format!("{:?}", e))?;
        } else if let Ok(window) = global_scope.dyn_into::<web_sys::Window>() {
            window
                .post_message(&js_value, "*")
                .map_err(|e| format!("{:?}", e))?;
        } else {
            return Err("Unknown global scope".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault::{IdentitySalts, Vault, VaultMetadata};
    use std::collections::HashMap;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn create_test_vault() -> Vault {
        Vault {
            metadata: VaultMetadata { peer_id: None },
            identity_salts: IdentitySalts::new(),
            username_pk: HashMap::new(),
            namespaces: HashMap::new(),
            sync_enabled: false,
        }
    }

    #[wasm_bindgen_test]
    fn test_notifier_creation() {
        let _notifier = Notifier::new();
    }

    #[wasm_bindgen_test]
    fn test_notify_vault_update_with_valid_data() {
        let notifier = Notifier::new();
        let vault = create_test_vault();

        let vault_data = serde_json::to_vec(&vault).unwrap();

        let result = notifier.notify_vault_update("test_vault", &vault_data);

        assert!(
            result.is_ok(),
            "Should successfully notify with valid vault data"
        );
    }

    #[wasm_bindgen_test]
    fn test_notify_vault_update_with_invalid_data() {
        let notifier = Notifier::new();

        let invalid_data = b"invalid json data";

        let result = notifier.notify_vault_update("test_vault", invalid_data);

        assert!(result.is_err(), "Should fail with invalid vault data");
        assert!(result.unwrap_err().contains("Failed to deserialize vault"));
    }

    #[wasm_bindgen_test]
    fn test_notify_vault_update_with_partial_json() {
        let notifier = Notifier::new();

        let partial_json = b"{\"metadata\": {}}";

        let result = notifier.notify_vault_update("test_vault", partial_json);

        assert!(result.is_err(), "Should fail with incomplete vault data");
    }

    #[wasm_bindgen_test]
    fn test_notify_with_complex_vault() {
        let notifier = Notifier::new();

        let mut vault = create_test_vault();
        vault.metadata.peer_id = Some("test_peer_123".to_string());

        let mut username_pk = HashMap::new();
        username_pk.insert("user1".to_string(), "pk1".to_string());
        username_pk.insert("user2".to_string(), "pk2".to_string());
        vault.username_pk = username_pk;

        vault.sync_enabled = true;

        let vault_data = serde_json::to_vec(&vault).unwrap();

        let result = notifier.notify_vault_update("complex_vault", &vault_data);

        assert!(
            result.is_ok(),
            "Should successfully notify with complex vault data"
        );
    }

    #[wasm_bindgen_test]
    fn test_notify_with_empty_vault_name() {
        let notifier = Notifier::new();
        let vault = create_test_vault();
        let vault_data = serde_json::to_vec(&vault).unwrap();

        let result = notifier.notify_vault_update("", &vault_data);

        assert!(result.is_ok(), "Should accept empty vault name");
    }
}
