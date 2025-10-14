#![cfg(target_arch = "wasm32")]
extern crate wasm_bindgen_test;

use hoddor::platform::Platform;
use hoddor::file_system::{get_root_directory_handle, remove_directory_with_contents};
use hoddor::vault::list_vaults;

use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub async fn cleanup_all_vaults() {
    let vaults = list_vaults().await.unwrap_or_else(|_| JsValue::from("[]"));
    let vault_list: Vec<String> = from_value(vaults).unwrap_or_default();

    let root = get_root_directory_handle()
        .await
        .expect("Failed to get root directory");

    for vault_name in vault_list {
        if let Err(e) = remove_directory_with_contents(&root, &vault_name).await {
            Platform::new().logger().log(&format!(
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
