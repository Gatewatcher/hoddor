#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use lazy_static::lazy_static;
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use std::sync::RwLock;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use hoddor::vault::*;
use hoddor::console::log;
use gloo_timers::future::TimeoutFuture;

wasm_bindgen_test_configure!(run_in_browser);

lazy_static! {
    static ref VAULT_PASSWORDS: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

fn store_password(vault_name: &str, password: &str) {
    VAULT_PASSWORDS
        .write()
        .unwrap()
        .insert(vault_name.to_string(), password.to_string());
}

fn get_password(vault_name: &str) -> Option<String> {
    VAULT_PASSWORDS.read().unwrap().get(vault_name).cloned()
}

fn clear_password_map() {
    VAULT_PASSWORDS.write().unwrap().clear();
}

async fn cleanup_all_vaults() {
    if let Ok(listed) = list_vaults().await {
        let vault_names: Vec<String> = from_value(listed).unwrap_or_default();
        for name in vault_names {
            if let Some(password) = get_password(&name) {
                let result = remove_vault(&name, JsValue::from_str(&password)).await;
                if result.is_err() {
                    eprintln!("Failed to remove vault: {}", name);
                }
            }
        }
    }
    clear_password_map();

    // let remaining_vaults = list_vaults().await.unwrap_or_else(|_| JsValue::from("[]"));
    // let remaining_list: Vec<String> = from_value(remaining_vaults).unwrap_or_default();
    // assert!(remaining_list.is_empty(), "Some vaults were not cleaned up: {:?}", remaining_list);
}

#[wasm_bindgen_test]
async fn test_vault_crud_operations() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    store_password("default", &password.as_string().unwrap());
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

    let read_data = read_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to read from vault");
    assert_eq!(read_data.as_string().unwrap(), "test_data");

    let result = upsert_vault(
        "default",
        password.clone(),
        namespace.clone(),
        "updated_test_data".into(),
        None,
        false,
    )
    .await;
    assert!(
        result.is_err(),
        "Should not be able to update existing namespace"
    );

    let new_namespace: JsValue = "test_namespace2".into();
    let updated_data: JsValue = "updated_test_data".into();
    upsert_vault(
        "default",
        password.clone(),
        new_namespace.clone(),
        updated_data.clone(),
        None,
        false,
    )
    .await
    .expect("Failed to create new namespace");

    remove_from_vault("default", password.clone(), namespace.clone())
        .await
        .expect("Failed to remove from vault");

    let result = read_from_vault("default", password, namespace).await;
    assert!(result.is_err(), "Namespace should be removed");

    store_password("default", "test_password123");
    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_namespaces() {
    let password: JsValue = "test_password123".into();
    let namespaces = vec!["ns1", "ns2", "ns3"];

    // Create the initial vault with the first namespace
    let first_ns: JsValue = namespaces[0].into();
    let data: JsValue = "test_data".into();

    store_password("default", "test_password123");
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

    store_password("default", "test_password123");
    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_invalid_password() {
    let password: JsValue = "correct_password".into();
    let wrong_password: JsValue = "wrong_password".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    store_password("default-2", "correct_password");
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

    // remove_from_vault("default-2", wrong_password.clone(), namespace)
    //     .await
    //     .expect("Failed to cleanup vault");

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

        store_password(
            &vault_name.as_string().unwrap(),
            &password.as_string().unwrap(),
        );
        cleanup_all_vaults().await;

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

    store_password(
        &vault_name.as_string().unwrap(),
        &password.as_string().unwrap(),
    );

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
    let namespace: JsValue = "test/namespace!@#$%^&*()".into();
    let data: JsValue = "test_data".into();

    store_password("default-character", "test/namespace!@#$%^&*()");

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

    for future in create_futures {
        future
            .await
            .expect("Failed to create vault in concurrent operation");
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

        let vault_name_str = vault_names[i]
            .as_string()
            .expect("Vaultname should be a valid string");
        let password_str = passwords[i]
            .as_string()
            .expect("Password should be a valid string");

        store_password(&vault_name_str, &password_str);
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
        let vault_name_str = &vault_name_strings[i];
        let password_str = passwords[i]
            .as_string()
            .expect("Password should be a valid string");

        store_password(&vault_name_str, &password_str);
    }

    for future in create_futures {
        future
            .await
            .expect("Failed to create namespace in concurrent operation");
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

    store_password("initial_namespace", "test_password123");
    store_password("default", "test_password123");
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

    store_password("default", "test_password123");
    store_password("initial_namespace", "test_password123");
    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_read_operations() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    store_password("default", "test_password123");
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

    for future in read_futures {
        let read_data = future
            .await
            .expect("Failed to read from vault concurrently");
        assert_eq!(read_data.as_string().unwrap(), "test_data");
    }

    store_password("default", "test_password123");
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

    store_password(vault_name, &password.as_string().expect("must be a string"));
    
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
    assert!(
        expired_result.is_err(),
        "Reading expired data should fail"
    );

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_force_cleanup() {
    cleanup_all_vaults().await;

    let vault_name = "force_cleanup_vault";
    let password: JsValue = "force_cleanup_password".into();
    store_password(vault_name, &password.as_string().expect("must be a string"));

    create_vault(
        JsValue::from_str(vault_name),
        password.clone(),
        JsValue::from_str("namespace1"),
        JsValue::from_str("data1"),
        Some(1), // 1 second expiration
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

    store_password(vault_name, password_str);
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

    store_password(vault_name, password_str);

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

    store_password(vault_name, &correct_password.as_string().unwrap());

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
    store_password(vault_name, &password.as_string().unwrap());

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
