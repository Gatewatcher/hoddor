#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use hoddor::{
    platform::Platform,
    facades::wasm::vault::{create_vault, read_from_vault, remove_vault, upsert_vault, vault_identity_from_passphrase},
};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
mod test_utils;

#[wasm_bindgen_test]
async fn performance_test_bulk_upserts() {
    let vault_name = "perf_test_vault";
    let password = "perf_password";
    let namespace_base = "bulk_namespace";
    let data_base = "bulk_data_";
    let platform = Platform::new();

    test_utils::cleanup_all_vaults().await;

    let t0 = platform.clock().now();
    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault for performance test");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(
        vault_name,
        &identity,
        "initial_ns",
        JsValue::from_str("initial_data"),
        None,
        false,
    )
    .await
    .expect("Failed to upsert initial data");

    let t1 = platform.clock().now();
    let vault_creation_time = t1 - t0;

    let num_upserts = 10;
    let t2 = platform.clock().now();
    for i in 0..num_upserts {
        let namespace = format!("{}{}", namespace_base, i);
        let data = format!("{}{}", data_base, i);
        upsert_vault(
            vault_name,
            &identity,
            &namespace,
            JsValue::from_str(&data),
            None,
            false,
        )
        .await
        .expect("Failed to upsert data in bulk");
    }
    let t3 = platform.clock().now();
    let upsert_time = t3 - t2;

    let t4 = platform.clock().now();
    for i in 0..num_upserts {
        let namespace = format!("{}{}", namespace_base, i);
        read_from_vault(vault_name, &identity, JsValue::from_str(&namespace))
            .await
            .expect("Failed to read data in bulk");
    }
    let t5 = platform.clock().now();
    let read_time = t5 - t4;

    remove_vault(vault_name)
        .await
        .expect("Failed to remove performance test vault");

    platform.logger().log(&format!(
        "Performance Report for Bulk Upserts:\n\
        Vault creation: {:.3}ms\n\
        Upserting {} namespaces: {:.3}ms\n\
        Reading {} namespaces: {:.3}ms\n",
        vault_creation_time, num_upserts, upsert_time, num_upserts, read_time
    ));

    platform.logger().log("Performance test for bulk upserts completed.");
}

#[wasm_bindgen_test]
async fn performance_test_large_data() {
    let vault_name = "perf_large_data_vault";
    let password = "perf_large_password";
    let namespace = "perf_large_namespace";
    let platform = Platform::new();

    test_utils::cleanup_all_vaults().await;

    let data_size_mb = 5;
    let data_size = data_size_mb * 1024 * 1024;
    let large_string = "X".repeat(data_size);
    let data = JsValue::from_str(&large_string);

    let t0 = platform.clock().now();
    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(
        vault_name,
        &identity,
        namespace,
        data.clone(),
        None,
        false,
    )
    .await
    .expect("Failed to create vault with large data");
    let t1 = platform.clock().now();
    let vault_creation_time = t1 - t0;

    let t2 = platform.clock().now();
    let read_data = read_from_vault(vault_name, &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read large data");
    let t3 = platform.clock().now();
    let read_time = t3 - t2;

    assert_eq!(
        read_data.as_string().unwrap().len(),
        data_size,
        "Data size mismatch in performance test"
    );

    remove_vault(vault_name)
        .await
        .expect("Failed to remove large data vault");

    platform.logger().log(&format!(
        "Performance Report for Large Data:\n\
        Vault creation with {} MB: {:.3}ms\n\
        Reading {} MB data: {:.3}ms\n",
        data_size_mb, vault_creation_time, data_size_mb, read_time
    ));

    platform.logger().log("Performance test for large data completed.");
}
