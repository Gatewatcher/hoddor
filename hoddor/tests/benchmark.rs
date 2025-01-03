#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use futures_util::future;
use gloo_timers::future::TimeoutFuture;
use hoddor::{
    console::log,
    measure::get_performance,
    vault::{
        create_vault,
        list_namespaces, list_vaults, read_from_vault, remove_from_vault, remove_vault,
        upsert_vault,
    },
};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;

wasm_bindgen_test_configure!(run_in_browser);
mod test_utils;

#[wasm_bindgen_test]
async fn performance_test_bulk_upserts() {
    let vault_name = "perf_test_vault";
    let password = JsValue::from_str("perf_password");
    let namespace_base = "bulk_namespace";
    let data_base = "bulk_data_";

    test_utils::cleanup_all_vaults().await;

    let t0 = get_performance().unwrap().now();
    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        JsValue::from_str("initial_ns"),
        JsValue::from_str("initial_data"),
        None,
    )
    .await
    .expect("Failed to create vault for performance test");
    let t1 = get_performance().unwrap().now();
    let vault_creation_time = t1 - t0;

    let num_upserts = 10;
    let t2 = get_performance().unwrap().now();
    for i in 0..num_upserts {
        let namespace = format!("{}{}", namespace_base, i);
        let data = format!("{}{}", data_base, i);
        upsert_vault(
            vault_name,
            password.clone(),
            JsValue::from_str(&namespace),
            JsValue::from_str(&data),
            None,
            false,
        )
        .await
        .expect("Failed to upsert data in bulk");
    }
    let t3 = get_performance().unwrap().now();
    let upsert_time = t3 - t2;

    let t4 = get_performance().unwrap().now();
    for i in 0..num_upserts {
        let namespace = format!("{}{}", namespace_base, i);
        read_from_vault(
            vault_name,
            password.clone(),
            JsValue::from_str(&namespace),
        )
        .await
        .expect("Failed to read data in bulk");
    }
    let t5 = get_performance().unwrap().now();
    let read_time = t5 - t4;

    remove_vault(vault_name, password.clone())
        .await
        .expect("Failed to remove performance test vault");

    log(&format!(
        "Performance Report for Bulk Upserts:\n\
        Vault creation: {:.3}ms\n\
        Upserting {} namespaces: {:.3}ms\n\
        Reading {} namespaces: {:.3}ms\n",
        vault_creation_time,
        num_upserts,
        upsert_time,
        num_upserts,
        read_time
    ));

    log("Performance test for bulk upserts completed.");
}

#[wasm_bindgen_test]
async fn performance_test_large_data() {
    let vault_name = "perf_large_data_vault";
    let password = JsValue::from_str("perf_large_password");
    let namespace = JsValue::from_str("perf_large_namespace");

    test_utils::cleanup_all_vaults().await;

    let data_size_mb = 5;
    let data_size = data_size_mb * 1024 * 1024;
    let large_string = "X".repeat(data_size);
    let data = JsValue::from_str(&large_string);

    let t0 = get_performance().unwrap().now();
    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault with large data");
    let t1 = get_performance().unwrap().now();
    let vault_creation_time = t1 - t0;

    let t2 = get_performance().unwrap().now();
    let read_data = read_from_vault(vault_name, password.clone(), namespace.clone())
        .await
        .expect("Failed to read large data");
    let t3 = get_performance().unwrap().now();
    let read_time = t3 - t2;

    assert_eq!(
        read_data.as_string().unwrap().len(),
        data_size,
        "Data size mismatch in performance test"
    );

    remove_vault(vault_name, password.clone())
        .await
        .expect("Failed to remove large data vault");

    log(&format!(
        "Performance Report for Large Data:\n\
        Vault creation with {} MB: {:.3}ms\n\
        Reading {} MB data: {:.3}ms\n",
        data_size_mb,
        vault_creation_time,
        data_size_mb,
        read_time
    ));

    log("Performance test for large data completed.");
}
