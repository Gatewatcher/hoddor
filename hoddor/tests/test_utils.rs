#![cfg(target_arch = "wasm32")]
extern crate wasm_bindgen_test;

use hoddor::facades::wasm::vault::list_vaults;
use hoddor::platform::Platform;

use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub async fn cleanup_all_vaults() {
    let platform = Platform::new();
    let storage = platform.storage();

    let vaults = list_vaults().await.unwrap_or_else(|_| JsValue::from("[]"));
    let vault_list: Vec<String> = from_value(vaults).unwrap_or_default();

    for vault_name in vault_list {
        if let Err(e) = storage.delete_directory(&vault_name).await {
            platform.logger().log(&format!(
                "Failed to remove vault directory {}: {:?}",
                vault_name, e
            ));
        }
    }

    let remaining_vaults = list_vaults().await.unwrap_or_else(|_| JsValue::from("[]"));
    let remaining_list: Vec<String> = from_value(remaining_vaults).unwrap_or_default();
    assert!(
        remaining_list.is_empty(),
        "Some vaults were not cleaned up: {:?}",
        remaining_list
    );
}
