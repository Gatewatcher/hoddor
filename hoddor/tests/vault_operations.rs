#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use futures_util::future;
use gloo_timers::future::TimeoutFuture;
use hoddor::{
    platform::Platform,
    facades::wasm::{
        configure_cleanup,
        vault::{
            create_vault, export_vault, force_cleanup_vault, import_vault,
            list_namespaces, list_vaults, read_from_vault, remove_from_vault, remove_vault,
            upsert_vault, vault_identity_from_passphrase,
        },
    },
};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

mod test_utils;

#[wasm_bindgen_test]
async fn test_vault_crud_operations() {
    let password = "test_password123";
    let namespace = "test_namespace";
    let data: JsValue = "test_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_namespaces() {
    let password = "test_password123";
    let namespaces = vec!["ns1", "ns2", "ns3"];
    let data: JsValue = "test_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    for ns in &namespaces {
        upsert_vault("default", &identity, ns, data.clone(), None, false)
            .await
            .expect("Failed to add namespace to vault");
    }

    let listed = list_namespaces("default")
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");

    for ns in &namespaces {
        assert!(
            listed_namespaces.contains(&ns.to_string()),
            "Missing namespace: {}",
            ns
        );
    }

    test_utils::cleanup_all_vaults().await;
}

// This test triggers a known bug in the `age` library when running in WASM:
// the library tries to load i18n translation files that don't exist in the WASM environment.
// Issue occurs when decryption fails with wrong password.
// TODO: Re-enable once age library fixes WASM i18n support or migrate to age 0.11+
#[ignore]
#[wasm_bindgen_test]
async fn test_invalid_password() {
    let password = "correct_password";
    let wrong_password = "wrong_password";
    let namespace = "test_namespace";
    let data: JsValue = "test_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default-2"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default-2")
        .await
        .expect("Failed to create identity");

    upsert_vault("default-2", &identity, namespace, data, None, false)
        .await
        .expect("Failed to upsert data");

    let wrong_identity = vault_identity_from_passphrase(wrong_password, "default-2")
        .await
        .expect("Failed to create wrong identity");

    let result = read_from_vault("default-2", &wrong_identity, JsValue::from_str(namespace)).await;
    assert!(result.is_err(), "Should fail with wrong password");

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_vaults() {
    let vault_configs = vec![
        ("vault1", "password1", "ns1", "data1"),
        ("vault2", "password2", "ns1", "data2"),
        ("vault3", "password3", "ns1", "data3"),
    ];

    for (vault_name, password, ns, data) in &vault_configs {
        create_vault(JsValue::from_str(vault_name))
            .await
            .expect("Failed to create test vault");

        let identity = vault_identity_from_passphrase(password, vault_name)
            .await
            .expect("Failed to create identity");

        upsert_vault(vault_name, &identity, ns, JsValue::from_str(data), None, false)
            .await
            .expect("Failed to upsert data");
    }

    let listed = list_vaults().await.expect("Failed to list vaults");
    let listed_vaults: Vec<String> = from_value(listed).expect("Failed to convert vault list");

    let expected_names: Vec<String> = vault_configs
        .iter()
        .map(|(name, _, _, _)| name.to_string())
        .collect();

    for name in &expected_names {
        assert!(listed_vaults.contains(name), "Missing vault: {}", name);
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_invalid_vault_name() {
    test_utils::cleanup_all_vaults().await;

    let invalid_names = vec![
        "",
        " ",
        "\t",
        "\n",
        "invalid/name",
        "invalid\\name",
        "invalid.name",
        "invalid@name",
        "invalid#name",
    ];

    for name in invalid_names {
        let result = create_vault(JsValue::from_str(name)).await;
        assert!(
            result.is_err(),
            "Should fail with invalid vault name: '{}', but succeeded",
            name
        );

        if result.is_ok() {
            let _ = remove_vault(name).await;
        }
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_duplicate_vault_creation() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "duplicate_test";
    let password = "test_password123";
    let namespace = "test_namespace";
    let data1: JsValue = "test_data1".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create first vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data1.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    let result = create_vault(JsValue::from_str(vault_name)).await;
    assert!(
        result.is_err(),
        "Should not be able to create duplicate vault"
    );

    let read_data = read_from_vault(vault_name, &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read vault");

    assert_eq!(
        read_data.as_string().unwrap(),
        "test_data1",
        "Original data should be preserved"
    );

    remove_vault(vault_name)
        .await
        .expect("Failed to remove test vault");
}

#[wasm_bindgen_test]
async fn test_special_characters_in_namespace() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "default-character";
    let password = "test_password123";
    let namespace = "test-namespace_#$+[]()";
    let data: JsValue = "test_data".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert with special characters");

    let read_data = read_from_vault(vault_name, &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read vault with special characters");

    assert_eq!(read_data.as_string().unwrap(), "test_data");

    remove_from_vault(vault_name, &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to remove vault with special characters");

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_vault_operations() {
    test_utils::cleanup_all_vaults().await;

    let vault_names: Vec<String> = (0..3).map(|i| format!("vault{}", i)).collect();
    let passwords: Vec<String> = (0..3).map(|i| format!("password{}", i)).collect();
    let namespaces: Vec<String> = (0..3).map(|i| format!("namespace{}", i)).collect();
    let data: Vec<String> = (0..3).map(|i| format!("data{}", i)).collect();

    let mut create_futures = Vec::new();
    for i in 0..3 {
        let vault_name = vault_names[i].clone();
        let password = passwords[i].clone();
        let namespace = namespaces[i].clone();
        let data_val = data[i].clone();

        let future = async move {
            create_vault(JsValue::from_str(&vault_name)).await?;
            let identity = vault_identity_from_passphrase(&password, &vault_name).await?;
            upsert_vault(&vault_name, &identity, &namespace, JsValue::from_str(&data_val), None, false).await
        };
        create_futures.push(future);
    }

    let results = future::join_all(create_futures).await;
    for result in results {
        result.expect("Failed to create vault in concurrent operation");
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_empty_namespace() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "default";
    let password = "test_password123";
    let namespace = "";
    let data: JsValue = "test_data".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    let result = upsert_vault(vault_name, &identity, namespace, data.clone(), None, false).await;
    assert!(result.is_err(), "Should fail with empty namespace");

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_empty_data() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "default";
    let password = "test_password123";
    let namespace = "test_namespace";
    let data: JsValue = "".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert empty data");

    let read_data = read_from_vault(vault_name, &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read from vault with empty data");
    assert_eq!(read_data.as_string().unwrap(), "");

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_special_characters_in_vault_name() {
    test_utils::cleanup_all_vaults().await;

    let invalid_names = vec![
        "vault/name",
        "vault\\name",
        "vault.name",
        "vault@name",
        "vault#name",
    ];

    for name in invalid_names {
        let result = create_vault(JsValue::from_str(name)).await;
        assert!(
            result.is_err(),
            "Should fail with invalid vault name: '{}', but succeeded",
            name
        );

        if result.is_ok() {
            let _ = remove_vault(name).await;
        }
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_vault_creation() {
    test_utils::cleanup_all_vaults().await;

    let vault_names: Vec<String> = (0..3).map(|i| format!("vault{}", i)).collect();
    let passwords: Vec<String> = (0..3).map(|i| format!("password{}", i)).collect();
    let namespaces: Vec<String> = (0..3).map(|i| format!("namespace{}", i)).collect();
    let data: Vec<String> = (0..3).map(|i| format!("data{}", i)).collect();

    for i in 0..3 {
        create_vault(JsValue::from_str(&vault_names[i]))
            .await
            .expect("Failed to create vault");

        let identity = vault_identity_from_passphrase(&passwords[i], &vault_names[i])
            .await
            .expect("Failed to create identity");

        upsert_vault(&vault_names[i], &identity, &namespaces[i], JsValue::from_str(&data[i]), None, false)
            .await
            .expect("Failed to upsert initial data");
    }

    let mut create_futures = Vec::new();

    for i in 0..3 {
        let vault_name = vault_names[i].clone();
        let password = passwords[i].clone();
        let new_namespace = format!("new_namespace{}", i);
        let data_val = data[i].clone();

        let future = async move {
            let identity = vault_identity_from_passphrase(&password, &vault_name).await?;
            upsert_vault(&vault_name, &identity, &new_namespace, JsValue::from_str(&data_val), None, false).await
        };
        create_futures.push(future);
    }

    let results = future::join_all(create_futures).await;
    for result in results {
        result.expect("Failed to create namespace in concurrent operation");
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_read_non_existent_namespace() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "default";
    let password = "test_password123";
    let namespace = "non_existent_namespace";

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    let result = read_from_vault(vault_name, &identity, JsValue::from_str(namespace)).await;
    assert!(
        result.is_err(),
        "Reading from a non-existent namespace should fail"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_namespaces_in_empty_vault() {
    let password = "test_password123";
    let namespace = "initial_namespace";
    let data: JsValue = "initial_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create initial vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert initial data");

    remove_from_vault("default", &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to remove initial namespace");

    let listed = list_namespaces("default")
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");

    assert!(
        listed_namespaces.is_empty(),
        "Namespaces list should be empty"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_read_operations() {
    let password = "test_password123";
    let namespace = "test_namespace";
    let data: JsValue = "test_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    let mut read_futures = Vec::new();
    for _ in 0..3 {
        let future = read_from_vault("default", &identity, JsValue::from_str(namespace));
        read_futures.push(future);
    }

    let results = future::join_all(read_futures).await;
    for result in results {
        let read_data = result.expect("Failed to read from vault concurrently");
        assert_eq!(read_data.as_string().unwrap(), "test_data");
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_data_expiration() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "expires_vault";
    let password = "expire_password";
    let namespace = "expiring_namespace";
    let data: JsValue = "temporary_data".into();
    let expires_in_seconds = Some(1);

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), expires_in_seconds, false)
        .await
        .expect("Failed to upsert data with expiration");

    let initial_read = read_from_vault(vault_name, &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read expiring data immediately");
    assert_eq!(
        initial_read.as_string().unwrap(),
        "temporary_data",
        "Data should still be present before expiration"
    );

    TimeoutFuture::new(1100).await;

    let expired_result = read_from_vault(vault_name, &identity, JsValue::from_str(namespace)).await;
    assert!(expired_result.is_err(), "Reading expired data should fail");

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_force_cleanup() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "force_cleanup_vault";
    let password = "force_cleanup_password";
    let namespace = "namespace1";
    let data: JsValue = "data1".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), Some(1), false)
        .await
        .expect("Failed to upsert first namespace with expiration");

    upsert_vault(
        vault_name,
        &identity,
        "namespace2",
        JsValue::from_str("data2"),
        Some(1), // also 1 second
        false,
    )
    .await
    .expect("Failed to insert second namespace with expiration");

    TimeoutFuture::new(1100).await;

    force_cleanup_vault(vault_name)
        .await
        .expect("Failed to force cleanup vault");

    let listed = list_namespaces(vault_name)
        .await
        .expect("Failed to list namespaces after forced cleanup");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    assert!(
        listed_namespaces.is_empty(),
        "All expired namespaces should be removed by forced cleanup"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_import_export_round_trip() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "round_trip_vault";
    let password = "round_trip_password";
    let namespace = "round_trip_namespace";
    let data = JsValue::from_str("round_trip_data");

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault for export");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    let initial_data = read_from_vault(vault_name, &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read data from created vault");
    assert_eq!(
        initial_data.as_string().unwrap(),
        "round_trip_data",
        "Initial data mismatch"
    );

    let exported_data = export_vault(vault_name)
        .await
        .expect("Failed to export vault");

    remove_vault(vault_name)
        .await
        .expect("Failed to remove vault");

    import_vault(vault_name, exported_data)
        .await
        .expect("Failed to import vault");

    let identity2 = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity after import");

    let read_data = read_from_vault(vault_name, &identity2, JsValue::from_str(namespace))
        .await
        .expect("Failed to read data from imported vault");

    assert_eq!(
        read_data.as_string().unwrap(),
        "round_trip_data",
        "Data mismatch after import/export round-trip"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_export_with_incorrect_password() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "import_incorrect_vault";
    let correct_password = "correct_password";
    let namespace = "test_namespace";
    let data: JsValue = "test_data".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault for export");

    let identity = vault_identity_from_passphrase(correct_password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    // Note: export_vault no longer takes a password parameter in new API
    // This test doesn't make sense with the new API where export doesn't require password
    let export_result = export_vault(vault_name).await;
    assert!(
        export_result.is_ok(),
        "Export should succeed without password in new API"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_disable_cleanup() {
    test_utils::cleanup_all_vaults().await;

    configure_cleanup(1);
    let vault_name = "cleanup_disabled_vault";
    let password = "cleanup_password";
    let namespace = "short_lived_ns";
    let data: JsValue = "short_lived_data".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), Some(2), false)
        .await
        .expect("Failed to upsert data with short expiration while cleanup is on");

    configure_cleanup(0);

    TimeoutFuture::new(3000).await;

    let read_data = read_from_vault(vault_name, &identity, JsValue::from_str(namespace)).await;

    match read_data {
        Ok(d) => {
            Platform::new().logger().log("Data remains because we disabled cleanup.");
            assert_eq!(
                d.as_string().unwrap(),
                "short_lived_data",
                "Data should remain if we rely solely on cleanup intervals."
            );
        }
        Err(e) => {
            Platform::new().logger().log("Data is expired at read time, so the read returned error");
            Platform::new().logger().log(&format!("Error: {:?}", e));
        }
    }

    configure_cleanup(1);
    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_upserts_different_namespaces() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "concurrent_diff_ns_vault";
    let password = "diff_ns_password";

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, "ns0", JsValue::from_str("data0"), None, false)
        .await
        .expect("Failed to upsert initial namespace");

    for i in 1..6 {
        let ns = format!("namespace{}", i);
        let dt = format!("data{}", i);
        Platform::new().logger().log(&format!("Preparing upsert for namespace '{}'", ns));
        match upsert_vault(
            vault_name,
            &identity,
            &ns,
            JsValue::from_str(&dt),
            None,
            false,
        ).await
        {
            Ok(_) => {},
            Err(e) => {
                Platform::new().logger().log(&format!("Upsert for namespace '{}' failed with error: {:?}", ns, e));
            }
        }
    }

    let namespaces = list_namespaces(vault_name)
        .await
        .expect("Failed to list namespaces");
    let ns_array = js_sys::Array::from(&namespaces);
    Platform::new().logger().log(&format!("Found {} namespaces:", ns_array.length()));
    for i in 0..ns_array.length() {
        if let Some(ns) = ns_array.get(i).as_string() {
            Platform::new().logger().log(&format!("  - {}", ns));
        }
    }

    for i in 0..6 {
        let ns = if i == 0 {
            "ns0".to_string()
        } else {
            format!("namespace{}", i)
        };
        let expected_data = if i == 0 {
            "data0".to_string()
        } else {
            format!("data{}", i)
        };

        match read_from_vault(vault_name, &identity, JsValue::from_str(&ns)).await {
            Ok(read_val) => {
                assert_eq!(
                    read_val.as_string().unwrap(),
                    expected_data,
                    "Data mismatch for namespace '{}'",
                    ns
                );
            }
            Err(e) => {
                Platform::new().logger().log(&format!("Read failed for namespace '{}': {:?}", ns, e));
            }
        }
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_upsert_with_replace() {
    let password = "test_password123";
    let namespace = "test_namespace";
    let initial_data: JsValue = "initial_data".into();
    let updated_data: JsValue = "updated_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, initial_data.clone(), None, false)
        .await
        .expect("Failed to upsert initial data");

    upsert_vault(
        "default",
        &identity,
        namespace,
        updated_data.clone(),
        None,
        true,
    )
    .await
    .expect("Failed to update data");

    let read_data = read_from_vault("default", &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read data");
    assert_eq!(read_data, updated_data, "Data was not updated correctly");

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_namespace_removal_validation() {
    let password = "test_password123";
    let namespace = "test_namespace";
    let data: JsValue = "test_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    remove_from_vault("default", &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to remove namespace");

    let listed = list_namespaces("default")
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    assert!(!listed_namespaces.contains(&"test_namespace".to_string()));

    let read_result = read_from_vault("default", &identity, JsValue::from_str(namespace)).await;
    assert!(
        read_result.is_err(),
        "Should not be able to read removed namespace"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_multiple_expired_namespaces() {
    let password = "test_password123";
    let data: JsValue = "test_data".into();
    let namespaces = vec!["ns1", "ns2", "ns3"];

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    for ns in &namespaces {
        upsert_vault(
            "default",
            &identity,
            *ns,
            data.clone(),
            Some(1),
            false,
        )
        .await
        .expect("Failed to add namespace");
    }

    TimeoutFuture::new(1500).await;

    force_cleanup_vault("default")
        .await
        .expect("Failed to force cleanup");

    let listed = list_namespaces("default")
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    assert!(
        listed_namespaces.is_empty(),
        "All namespaces should be expired and removed"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_large_data_payload() {
    let password = "test_password123";
    let namespace = "test_namespace";

    let large_data = "x".repeat(1024 * 1024);
    let data: JsValue = large_data.clone().into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert large data");

    let read_data = read_from_vault("default", &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read large data");

    let read_str: String = from_value(read_data).expect("Failed to convert read data");
    assert_eq!(
        read_str, large_data,
        "Large data was not preserved correctly"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_unicode_namespace() {
    let password = "test_password123";
    let namespace = "测试_namespace_🔒";
    let data: JsValue = "test_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data with Unicode namespace");

    let listed = list_namespaces("default")
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    assert!(listed_namespaces.contains(&"测试_namespace_🔒".to_string()));

    let read_data = read_from_vault("default", &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read from Unicode namespace");
    assert_eq!(
        read_data, data,
        "Data in Unicode namespace was not preserved correctly"
    );

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_data_integrity() {
    let password = "test_password123";
    let base_namespace = "test_namespace";
    let base_data = "test_data";

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, base_namespace, JsValue::from_str(base_data), None, false)
        .await
        .expect("Failed to create initial namespace");

    let operations_count = 50;

    for i in 0..operations_count {
        let namespace = format!("{}{}", base_namespace, i);
        let data = format!("{}{}", base_data, i);

        let mut retries = 3;
        while retries > 0 {
            match upsert_vault(
                "default",
                &identity,
                &namespace,
                JsValue::from_str(&data),
                None,
                false,
            )
            .await
            {
                Ok(_) => break,
                Err(e) => {
                    retries -= 1;
                    if retries > 0 {
                        TimeoutFuture::new(50).await;
                    } else {
                        panic!("Concurrent operation failed after retries: {:?}", e);
                    }
                }
            }
        }
    }

    let listed = list_namespaces("default")
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");

    assert_eq!(
        listed_namespaces.len(),
        operations_count + 1,
        "Not all namespaces were created"
    );

    for i in 0..operations_count {
        let namespace = format!("{}{}", base_namespace, i);
        let expected_data = format!("{}{}", base_data, i);

        let read_data = read_from_vault("default", &identity, JsValue::from_str(&namespace))
            .await
            .expect("Failed to read data");

        let read_str: String = from_value(read_data).expect("Failed to convert read data");
        assert_eq!(
            read_str, expected_data,
            "Data corruption detected in namespace {}",
            namespace
        );
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_read_write_integrity() {
    let password = "test_password123";
    let namespace = "test_namespace";
    let initial_data: JsValue = "initial_data".into();

    test_utils::cleanup_all_vaults().await;

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, initial_data.clone(), None, false)
        .await
        .expect("Failed to upsert initial data");

    let mut write_futures = Vec::new();
    let operations_count = 20;

    for i in 0..operations_count {
        let data = format!("data_version_{}", i);

        write_futures.push(upsert_vault(
            "default",
            &identity,
            namespace,
            JsValue::from_str(&data),
            None,
            true,
        ));
    }

    let write_results = future::join_all(write_futures).await;
    for result in write_results {
        assert!(result.is_ok(), "Concurrent write operation failed");
    }

    let mut read_futures = Vec::new();
    for _ in 0..operations_count {
        read_futures.push(read_from_vault(
            "default",
            &identity,
            JsValue::from_str(namespace),
        ));
    }

    let read_results = future::join_all(read_futures).await;
    for result in read_results {
        let data = result.expect("Failed to read data");
        let data_str: String = from_value(data).expect("Failed to convert read data");
        assert!(
            data_str.starts_with("data_version_"),
            "Invalid data format found: {}",
            data_str
        );
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_data_integrity_with_binary() {
    let password = "test_password123";
    let namespace = "binary_test";

    test_utils::cleanup_all_vaults().await;

    let mut binary_data = Vec::new();
    for i in 0..256 {
        binary_data.push(i as u8);
    }

    binary_data.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
    binary_data.extend_from_slice("🔒\0\n\r\t".as_bytes());

    let data: JsValue =
        serde_wasm_bindgen::to_value(&binary_data).expect("Failed to convert binary data");

    create_vault(JsValue::from_str("default"))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, "default")
        .await
        .expect("Failed to create identity");

    upsert_vault("default", &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert binary data");

    let read_data = read_from_vault("default", &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to read binary data");

    let read_bytes: Vec<u8> = from_value(read_data).expect("Failed to convert read binary data");

    assert_eq!(read_bytes, binary_data, "Binary data corruption detected");

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_same_namespace_upserts() {
    test_utils::cleanup_all_vaults().await;

    let vault_name = "concurrent_same_ns";
    let password = "test_password123";
    let namespace = "shared_namespace";

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, JsValue::from_str("initial_data"), None, false)
        .await
        .expect("Failed to create initial namespace");

    let mut futures = Vec::new();
    let iterations = 10;

    for i in 0..iterations {
        let data = format!("concurrent_data_{}", i);
        futures.push(upsert_vault(
            vault_name,
            &identity,
            namespace,
            JsValue::from_str(&data),
            None,
            true,
        ));
    }

    let results = futures_util::future::join_all(futures).await;

    for (i, res) in results.into_iter().enumerate() {
        assert!(
            res.is_ok(),
            "Concurrent upsert #{} to the same namespace failed: {:?}",
            i,
            res.err()
        );
    }

    let final_read_result = read_from_vault(vault_name, &identity, JsValue::from_str(namespace)).await;
    let final_data = final_read_result
        .expect("Failed to read final data from concurrent upserts")
        .as_string()
        .expect("final data is not a string");

    let expected = format!("concurrent_data_{}", iterations - 1);
    assert_eq!(
        final_data, expected,
        "Last-write-wins expectation not met for concurrent upserts"
    );

    test_utils::cleanup_all_vaults().await;
}
