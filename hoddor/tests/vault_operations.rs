#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use futures_util::future;
use gloo_timers::future::TimeoutFuture;
use hoddor::{
    console::log,
    file_system::{get_root_directory_handle, remove_directory_with_contents},
    vault::{
        configure_cleanup, create_vault, export_vault, force_cleanup_vault, import_vault,
        list_namespaces, list_vaults, read_from_vault, remove_from_vault, remove_vault,
        upsert_vault,
    },
};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

async fn cleanup_all_vaults() {
    let vaults = list_vaults().await.unwrap_or_else(|_| JsValue::from("[]"));
    let vault_list: Vec<String> = from_value(vaults).unwrap_or_default();
    log(&format!("Found {} vaults to clean up", vault_list.len()));

    let root = get_root_directory_handle()
        .await
        .expect("Failed to get root directory");

    for vault_name in vault_list {
        if let Err(e) = remove_directory_with_contents(&root, &vault_name).await {
            log(&format!(
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

#[wasm_bindgen_test]
async fn test_vault_crud_operations() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    cleanup_all_vaults().await;
    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_namespaces() {
    let password: JsValue = "test_password123".into();
    let namespaces = vec!["ns1", "ns2", "ns3"];

    let first_ns: JsValue = namespaces[0].into();
    let data: JsValue = "test_data".into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        first_ns,
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create initial vault");

    for ns in &namespaces[1..] {
        let ns_value: JsValue = (*ns).into();
        upsert_vault(
            "default",
            password.clone(),
            ns_value,
            data.clone(),
            None,
            false,
        )
        .await
        .expect("Failed to add namespace to vault");
    }

    let listed = list_namespaces("default", password.clone())
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

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_invalid_password() {
    let password: JsValue = "correct_password".into();
    let wrong_password: JsValue = "wrong_password".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default-2"),
        password.clone(),
        namespace.clone(),
        data,
        None,
    )
    .await
    .expect("Failed to create vault");

    let result = read_from_vault("default-2", wrong_password.clone(), namespace.clone()).await;
    assert!(result.is_err(), "Should fail with wrong password");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_vaults() {
    let vault_configs = vec![
        ("vault1", "password1", "ns1", "data1"),
        ("vault2", "password2", "ns1", "data2"),
        ("vault3", "password3", "ns1", "data3"),
    ];

    for (vault_name, password, ns, data) in &vault_configs {
        let vault_name: JsValue = (*vault_name).into();
        let password: JsValue = (*password).into();
        let namespace: JsValue = (*ns).into();
        let data: JsValue = (*data).into();

        create_vault(vault_name, password, namespace, data, None)
            .await
            .expect("Failed to create test vault");
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

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_invalid_vault_name() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();
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
        let invalid_name: JsValue = name.into();
        let result = create_vault(
            invalid_name.clone(),
            password.clone(),
            namespace.clone(),
            data.clone(),
            None,
        )
        .await;
        assert!(
            result.is_err(),
            "Should fail with invalid vault name: '{}', but succeeded",
            name
        );

        if let Ok(_) = result {
            let _ = remove_vault(name, password.clone()).await;
        }
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_duplicate_vault_creation() {
    cleanup_all_vaults().await;

    let vault_name: JsValue = "duplicate_test".into();
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data1: JsValue = "test_data1".into();
    let data2: JsValue = "test_data2".into();

    create_vault(
        vault_name.clone(),
        password.clone(),
        namespace.clone(),
        data1.clone(),
        None,
    )
    .await
    .expect("Failed to create first vault");

    let result = create_vault(
        vault_name.clone(),
        password.clone(),
        namespace.clone(),
        data2.clone(),
        None,
    )
    .await;
    assert!(
        result.is_err(),
        "Should not be able to create duplicate vault"
    );

    let read_data = read_from_vault("duplicate_test", password.clone(), namespace.clone())
        .await
        .expect("Failed to read vault");

    assert_eq!(
        read_data.as_string().unwrap(),
        "test_data1",
        "Original data should be preserved"
    );

    remove_vault("duplicate_test", password.clone())
        .await
        .expect("Failed to remove test vault");
}

#[wasm_bindgen_test]
async fn test_special_characters_in_namespace() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test-namespace_#$+[]()".into();
    let data: JsValue = "test_data".into();

    create_vault(
        JsValue::from_str("default-character"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault with special characters");

    let read_data = read_from_vault("default-character", password.clone(), namespace.clone())
        .await
        .expect("Failed to read vault with special characters");

    assert_eq!(read_data.as_string().unwrap(), "test_data");

    remove_from_vault("default-character", password, namespace)
        .await
        .expect("Failed to remove vault with special characters");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_vault_operations() {
    cleanup_all_vaults().await;

    let passwords: Vec<JsValue> = (0..3).map(|i| format!("password{}", i).into()).collect();
    let namespaces: Vec<JsValue> = (0..3).map(|i| format!("namespace{}", i).into()).collect();
    let data: Vec<JsValue> = (0..3).map(|i| format!("data{}", i).into()).collect();
    let vault_names: Vec<JsValue> = (0..3).map(|i| format!("vault{}", i).into()).collect();

    let mut create_futures = Vec::new();
    for i in 0..3 {
        let future = create_vault(
            vault_names[i].clone(),
            passwords[i].clone(),
            namespaces[i].clone(),
            data[i].clone(),
            None,
        );
        create_futures.push(future);
    }

    let results = future::join_all(create_futures).await;
    for result in results {
        result.expect("Failed to create vault in concurrent operation");
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_empty_namespace() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "".into();
    let data: JsValue = "test_data".into();

    let result = create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await;
    assert!(result.is_err(), "Should fail with empty namespace");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_empty_data() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "".into();

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault with empty data");

    let read_data = read_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to read from vault with empty data");
    assert_eq!(read_data.as_string().unwrap(), "");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_special_characters_in_vault_name() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();
    let invalid_names = vec![
        "vault/name",
        "vault\\name",
        "vault.name",
        "vault@name",
        "vault#name",
    ];

    for name in invalid_names {
        let invalid_name: JsValue = name.into();
        let result = create_vault(
            invalid_name.clone(),
            password.clone(),
            namespace.clone(),
            data.clone(),
            None,
        )
        .await;
        assert!(
            result.is_err(),
            "Should fail with invalid vault name: '{}', but succeeded",
            name
        );

        if let Ok(_) = result {
            let _ = remove_vault(name, password.clone()).await;
        }
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_vault_creation() {
    cleanup_all_vaults().await;

    let vault_names: Vec<JsValue> = (0..3).map(|i| format!("vault{}", i).into()).collect();
    let passwords: Vec<JsValue> = (0..3).map(|i| format!("password{}", i).into()).collect();
    let namespaces: Vec<JsValue> = (0..3).map(|i| format!("namespace{}", i).into()).collect();
    let data: Vec<JsValue> = (0..3).map(|i| format!("data{}", i).into()).collect();

    for i in 0..3 {
        create_vault(
            vault_names[i].clone(),
            passwords[i].clone(),
            namespaces[i].clone(),
            data[i].clone(),
            None,
        )
        .await
        .expect("Failed to create vault");

        vault_names[i]
            .as_string()
            .expect("Vaultname should be a valid string");
        passwords[i]
            .as_string()
            .expect("Password should be a valid string");
    }

    let vault_name_strings: Vec<String> = (0..3).map(|i| format!("vault{}", i)).collect();
    let mut create_futures = Vec::new();

    for i in 0..3 {
        let new_namespace: JsValue = format!("new_namespace{}", i).into();
        let future = upsert_vault(
            &vault_name_strings[i],
            passwords[i].clone(),
            new_namespace.clone(),
            data[i].clone(),
            None,
            false,
        );
        create_futures.push(future);
        passwords[i]
            .as_string()
            .expect("Password should be a valid string");
    }

    let results = future::join_all(create_futures).await;
    for result in results {
        result.expect("Failed to create namespace in concurrent operation");
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_read_non_existent_namespace() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "non_existent_namespace".into();

    let result = read_from_vault("default", password, namespace).await;
    assert!(
        result.is_err(),
        "Reading from a non-existent namespace should fail"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_namespaces_in_empty_vault() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "initial_namespace".into();
    let data: JsValue = "initial_data".into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create initial vault");

    remove_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to remove initial namespace");

    let listed = list_namespaces("default", password)
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");

    assert!(
        listed_namespaces.is_empty(),
        "Namespaces list should be empty"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_read_operations() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault");

    let mut read_futures = Vec::new();
    for _ in 0..3 {
        let future = read_from_vault("default", password.clone(), namespace.clone());
        read_futures.push(future);
    }

    let results = future::join_all(read_futures).await;
    for result in results {
        let read_data = result.expect("Failed to read from vault concurrently");
        assert_eq!(read_data.as_string().unwrap(), "test_data");
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_data_expiration() {
    cleanup_all_vaults().await;

    let vault_name = "expires_vault";
    let password: JsValue = "expire_password".into();
    let namespace: JsValue = "expiring_namespace".into();
    let data: JsValue = "temporary_data".into();
    let expires_in_seconds = Some(1);

    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        namespace.clone(),
        data.clone(),
        expires_in_seconds,
    )
    .await
    .expect("Failed to create vault with expiration");

    let initial_read = read_from_vault(vault_name, password.clone(), namespace.clone())
        .await
        .expect("Failed to read expiring data immediately");
    assert_eq!(
        initial_read.as_string().unwrap(),
        "temporary_data",
        "Data should still be present before expiration"
    );

    TimeoutFuture::new(1100).await;

    let expired_result = read_from_vault(vault_name, password.clone(), namespace.clone()).await;
    assert!(expired_result.is_err(), "Reading expired data should fail");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_force_cleanup() {
    cleanup_all_vaults().await;

    let vault_name = "force_cleanup_vault";
    let password: JsValue = "force_cleanup_password".into();
    let namespace: JsValue = "namespace1".into();
    let data: JsValue = "data1".into();

    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        namespace.clone(),
        data.clone(),
        Some(1),
    )
    .await
    .expect("Failed to create initial vault with expiration");

    upsert_vault(
        vault_name,
        password.clone(),
        JsValue::from_str("namespace2"),
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

    let listed = list_namespaces(vault_name, password.clone())
        .await
        .expect("Failed to list namespaces after forced cleanup");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    assert!(
        listed_namespaces.is_empty(),
        "All expired namespaces should be removed by forced cleanup"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_import_export_round_trip() {
    cleanup_all_vaults().await;

    let vault_name = "round_trip_vault";
    let password_str = "round_trip_password";
    let password = JsValue::from_str(password_str);
    let namespace = JsValue::from_str("round_trip_namespace");
    let data = JsValue::from_str("round_trip_data");

    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault for export");

    let initial_data = read_from_vault(vault_name, password.clone(), namespace.clone())
        .await
        .expect("Failed to read data from created vault");
    assert_eq!(
        initial_data.as_string().unwrap(),
        "round_trip_data",
        "Initial data mismatch"
    );

    let exported_data = export_vault(vault_name, password.clone())
        .await
        .expect("Failed to export vault");

    remove_vault(vault_name, password.clone())
        .await
        .expect("Failed to remove vault");

    import_vault(vault_name, exported_data)
        .await
        .expect("Failed to import vault");

    let read_data = read_from_vault(vault_name, password.clone(), namespace.clone())
        .await
        .expect("Failed to read data from imported vault");

    assert_eq!(
        read_data.as_string().unwrap(),
        "round_trip_data",
        "Data mismatch after import/export round-trip"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_export_with_incorrect_password() {
    cleanup_all_vaults().await;

    let vault_name = "import_incorrect_vault";
    let correct_password: JsValue = "correct_password".into();
    let incorrect_password: JsValue = "incorrect_password".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    create_vault(
        JsValue::from_str(vault_name),
        correct_password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault for export");

    let export_result = export_vault(vault_name, incorrect_password.clone()).await;
    assert!(
        export_result.is_err(),
        "Export should fail when using an incorrect password"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_disable_cleanup() {
    cleanup_all_vaults().await;

    configure_cleanup(1);
    let vault_name = "cleanup_disabled_vault";
    let password: JsValue = "cleanup_password".into();
    let namespace: JsValue = "short_lived_ns".into();
    let data: JsValue = "short_lived_data".into();

    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        namespace.clone(),
        data.clone(),
        Some(2),
    )
    .await
    .expect("Failed to create vault with short expiration while cleanup is on");

    configure_cleanup(0);

    TimeoutFuture::new(3000).await;

    let read_data = read_from_vault(vault_name, password.clone(), namespace.clone()).await;

    match read_data {
        Ok(d) => {
            log("Data remains because we disabled cleanup.");
            assert_eq!(
                d.as_string().unwrap(),
                "short_lived_data",
                "Data should remain if we rely solely on cleanup intervals."
            );
        }
        Err(e) => {
            log("Data is expired at read time, so the read returned error");
            log(&format!("Error: {:?}", e));
        }
    }

    configure_cleanup(1);
    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_upserts_different_namespaces() {
    cleanup_all_vaults().await;

    let vault_name = "concurrent_diff_ns_vault";
    let password_str = "diff_ns_password";
    let password = JsValue::from_str(password_str);

    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        JsValue::from_str("ns0"),
        JsValue::from_str("data0"),
        None,
    )
    .await
    .expect("Failed to create vault");

    let mut tasks = vec![];
    for i in 1..6 {
        let ns = format!("namespace{}", i);
        let dt = format!("data{}", i);
        log(&format!("Preparing upsert for namespace '{}'", ns));
        tasks.push(upsert_vault(
            vault_name,
            password.clone(),
            JsValue::from_str(&ns),
            JsValue::from_str(&dt),
            None,
            false,
        ));
    }

    let results = future::join_all(tasks).await;
    for (i, result) in results.into_iter().enumerate() {
        if let Err(e) = result {
            log(&format!("Upsert #{} failed with error: {:?}", i, e));
        }
    }

    let namespaces = list_namespaces(vault_name, password.clone())
        .await
        .expect("Failed to list namespaces");
    let ns_array = js_sys::Array::from(&namespaces);
    log(&format!("Found {} namespaces:", ns_array.length()));
    for i in 0..ns_array.length() {
        if let Some(ns) = ns_array.get(i).as_string() {
            log(&format!("  - {}", ns));
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

        match read_from_vault(vault_name, password.clone(), JsValue::from_str(&ns)).await {
            Ok(read_val) => {
                assert_eq!(
                    read_val.as_string().unwrap(),
                    expected_data,
                    "Data mismatch for namespace '{}'",
                    ns
                );
            }
            Err(e) => {
                log(&format!("Read failed for namespace '{}': {:?}", ns, e));
            }
        }
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_upsert_with_replace() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let initial_data: JsValue = "initial_data".into();
    let updated_data: JsValue = "updated_data".into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        initial_data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault");

    upsert_vault(
        "default",
        password.clone(),
        namespace.clone(),
        updated_data.clone(),
        None,
        true,
    )
    .await
    .expect("Failed to update data");

    let read_data = read_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to read data");
    assert_eq!(read_data, updated_data, "Data was not updated correctly");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_namespace_removal_validation() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault");

    remove_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to remove namespace");

    let listed = list_namespaces("default", password.clone())
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    assert!(!listed_namespaces.contains(&"test_namespace".to_string()));

    let read_result = read_from_vault("default", password.clone(), namespace.clone()).await;
    assert!(
        read_result.is_err(),
        "Should not be able to read removed namespace"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_multiple_expired_namespaces() {
    let password: JsValue = "test_password123".into();
    let data: JsValue = "test_data".into();
    let namespaces = vec!["ns1", "ns2", "ns3"];

    cleanup_all_vaults().await;

    for ns in &namespaces {
        let ns_value: JsValue = (*ns).into();
        if ns == &namespaces[0] {
            create_vault(
                JsValue::from_str("default"),
                password.clone(),
                ns_value,
                data.clone(),
                Some(1),
            )
            .await
            .expect("Failed to create vault");
        } else {
            upsert_vault(
                "default",
                password.clone(),
                ns_value,
                data.clone(),
                Some(1),
                false,
            )
            .await
            .expect("Failed to add namespace");
        }
    }

    TimeoutFuture::new(1500).await;

    force_cleanup_vault("default")
        .await
        .expect("Failed to force cleanup");

    let listed = list_namespaces("default", password.clone())
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    assert!(
        listed_namespaces.is_empty(),
        "All namespaces should be expired and removed"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_large_data_payload() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();

    let large_data = "x".repeat(1024 * 1024);
    let data: JsValue = large_data.clone().into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault with large data");

    let read_data = read_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to read large data");

    let read_str: String = from_value(read_data).expect("Failed to convert read data");
    assert_eq!(
        read_str, large_data,
        "Large data was not preserved correctly"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_unicode_namespace() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "测试_namespace_🔒".into();
    let data: JsValue = "test_data".into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault with Unicode namespace");

    let listed = list_namespaces("default", password.clone())
        .await
        .expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    assert!(listed_namespaces.contains(&"测试_namespace_🔒".to_string()));

    let read_data = read_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to read from Unicode namespace");
    assert_eq!(
        read_data, data,
        "Data in Unicode namespace was not preserved correctly"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_data_integrity() {
    let password: JsValue = "test_password123".into();
    let base_namespace = "test_namespace";
    let base_data = "test_data";

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        JsValue::from_str(base_namespace),
        JsValue::from_str(base_data),
        None,
    )
    .await
    .expect("Failed to create initial vault");

    let mut futures = Vec::new();
    let operations_count = 50;

    for i in 0..operations_count {
        let namespace = format!("{}{}", base_namespace, i);
        let data = format!("{}{}", base_data, i);
        let ns_value: JsValue = namespace.into();
        let data_value: JsValue = data.into();
        let password = password.clone();

        futures.push(upsert_vault(
            "default",
            password.clone(),
            ns_value,
            data_value,
            None,
            false,
        ));
    }

    future::join_all(futures).await;

    let listed = list_namespaces("default", password.clone())
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

        let read_data = read_from_vault("default", password.clone(), JsValue::from_str(&namespace))
            .await
            .expect("Failed to read data");

        let read_str: String = from_value(read_data).expect("Failed to convert read data");
        assert_eq!(
            read_str, expected_data,
            "Data corruption detected in namespace {}",
            namespace
        );
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_read_write_integrity() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let initial_data: JsValue = "initial_data".into();

    cleanup_all_vaults().await;

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        initial_data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault");

    let mut write_futures = Vec::new();
    let operations_count = 20;

    for i in 0..operations_count {
        let password = password.clone();
        let namespace = namespace.clone();
        let data = format!("data_version_{}", i);

        write_futures.push(upsert_vault(
            "default",
            password.clone(),
            namespace.clone(),
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
            password.clone(),
            namespace.clone(),
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

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_data_integrity_with_binary() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "binary_test".into();

    cleanup_all_vaults().await;

    let mut binary_data = Vec::new();
    for i in 0..256 {
        binary_data.push(i as u8);
    }

    binary_data.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
    binary_data.extend_from_slice("🔒\0\n\r\t".as_bytes());

    let data: JsValue =
        serde_wasm_bindgen::to_value(&binary_data).expect("Failed to convert binary data");

    create_vault(
        JsValue::from_str("default"),
        password.clone(),
        namespace.clone(),
        data.clone(),
        None,
    )
    .await
    .expect("Failed to create vault with binary data");

    let read_data = read_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to read binary data");

    let read_bytes: Vec<u8> = from_value(read_data).expect("Failed to convert read binary data");

    assert_eq!(read_bytes, binary_data, "Binary data corruption detected");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_same_namespace_upserts() {
    cleanup_all_vaults().await;

    let vault_name = "concurrent_same_ns";
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "shared_namespace".into();

    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        namespace.clone(),
        JsValue::from_str("initial_data"),
        None,
    )
    .await
    .expect("Failed to create initial vault");

    let mut futures = Vec::new();
    let iterations = 10;

    for i in 0..iterations {
        let data = format!("concurrent_data_{}", i);
        futures.push(upsert_vault(
            vault_name,
            password.clone(),
            namespace.clone(),
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

    let final_read_result = read_from_vault(vault_name, password.clone(), namespace.clone()).await;
    let final_data = final_read_result
        .expect("Failed to read final data from concurrent upserts")
        .as_string()
        .expect("final data is not a string");

    let expected = format!("concurrent_data_{}", iterations - 1);
    assert_eq!(
        final_data, expected,
        "Last-write-wins expectation not met for concurrent upserts"
    );

    cleanup_all_vaults().await;
}
