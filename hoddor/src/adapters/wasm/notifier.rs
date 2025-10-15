use crate::ports::NotifierPort;
use crate::global::get_global_scope;
use crate::notifications;
use wasm_bindgen::JsCast;

/// WASM notifier adapter using postMessage API.
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

        let vault: crate::vault::Vault = serde_json::from_slice(vault_data)
            .map_err(|e| format!("Failed to deserialize vault: {}", e))?;

        let msg = notifications::Message {
            event: notifications::EventType::VaultUpdate,
            data: vault,
        };

        let js_value = serde_wasm_bindgen::to_value(&msg)
            .map_err(|e| format!("Failed to serialize: {:?}", e))?;

        if let Ok(worker_scope) = global_scope.clone().dyn_into::<web_sys::DedicatedWorkerGlobalScope>() {
            worker_scope.post_message(&js_value).map_err(|e| format!("{:?}", e))?;
        } else if let Ok(window) = global_scope.dyn_into::<web_sys::Window>() {
            window.post_message(&js_value, "*").map_err(|e| format!("{:?}", e))?;
        } else {
            return Err("Unknown global scope".to_string());
        }

        Ok(())
    }
}
