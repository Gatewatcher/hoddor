#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;
use hoddor::*;
use wasm_bindgen::JsValue;
use serde_wasm_bindgen::from_value;

wasm_bindgen_test_configure!(run_in_browser);

async fn cleanup_all_vaults() {
    if let Ok(listed) = list_vaults().await {
        let vault_names: Vec<String> = from_value(listed).unwrap_or_default();
        for name in vault_names {
            let _ = remove_vault_with_name(&name).await;
        }
    }
    let _ = remove_vault().await;
}

#[wasm_bindgen_test]
async fn test_vault_crud_operations() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    create_vault(password.clone(), namespace.clone(), data.clone()).await.expect("Failed to create vault");

    let read_data = read_from_vault(password.clone(), namespace.clone()).await.expect("Failed to read from vault");
    assert_eq!(read_data.as_string().unwrap(), "test_data");

    let updated_data: JsValue = "updated_test_data".into();
    upsert_vault(password.clone(), namespace.clone(), updated_data.clone()).await.expect("Failed to update vault");

    let read_updated_data = read_from_vault(password.clone(), namespace.clone()).await.expect("Failed to read updated data");
    assert_eq!(read_updated_data.as_string().unwrap(), "updated_test_data");

    remove_from_vault(password.clone(), namespace.clone()).await.expect("Failed to remove from vault");

    let result = read_from_vault(password, namespace).await;
    assert!(result.is_err(), "Vault should be removed");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_namespaces() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespaces = vec!["ns1", "ns2", "ns3"];

    for ns in &namespaces {
        let ns_value: JsValue = (*ns).into();
        let data: JsValue = "test_data".into();
        create_vault(password.clone(), ns_value, data).await.expect("Failed to create vault");
    }

    let listed = list_namespaces(password.clone()).await.expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");
    
    for ns in &namespaces {
        assert!(listed_namespaces.contains(&ns.to_string()), "Missing namespace: {}", ns);
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_invalid_password() {
    let password: JsValue = "correct_password".into();
    let wrong_password: JsValue = "wrong_password".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    create_vault(password.clone(), namespace.clone(), data).await.expect("Failed to create vault");

    let result = read_from_vault(wrong_password, namespace.clone()).await;
    assert!(result.is_err(), "Should fail with wrong password");

    remove_from_vault(password, namespace).await.expect("Failed to cleanup vault");
}

#[wasm_bindgen_test]
async fn test_list_vaults() {
    cleanup_all_vaults().await;

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
        
        create_vault_with_name(vault_name, password, namespace, data)
            .await
            .expect("Failed to create test vault");
    }

    let listed = list_vaults().await.expect("Failed to list vaults");
    let listed_vaults: Vec<String> = from_value(listed).expect("Failed to convert vault list");
    
    let expected_names: Vec<String> = vault_configs.iter()
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
        let result = create_vault_with_name(invalid_name.clone(), password.clone(), namespace.clone(), data.clone()).await;
        assert!(
            result.is_err(),
            "Should fail with invalid vault name: '{}', but succeeded",
            name
        );
        
        if let Ok(_) = result {
            let _ = remove_vault_with_name(name).await;
        }
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_duplicate_vault_creation() {
    let vault_name: JsValue = "duplicate_test".into();
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data1: JsValue = "test_data1".into();
    let data2: JsValue = "test_data2".into();

    create_vault_with_name(vault_name.clone(), password.clone(), namespace.clone(), data1)
        .await
        .expect("Failed to create first vault");

    create_vault_with_name(vault_name.clone(), password.clone(), namespace.clone(), data2)
        .await
        .expect("Failed to create second vault");

    let read_data = read_from_vault_with_name("duplicate_test", password.clone(), namespace.clone())
        .await
        .expect("Failed to read vault");
    
    assert_eq!(read_data.as_string().unwrap(), "test_data2", "Data should be overwritten");

    remove_vault_with_name("duplicate_test")
        .await
        .expect("Failed to remove test vault");
}

#[wasm_bindgen_test]
async fn test_special_characters_in_namespace() {
    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test/namespace!@#$%^&*()".into();
    let data: JsValue = "test_data".into();

    create_vault(password.clone(), namespace.clone(), data.clone())
        .await
        .expect("Failed to create vault with special characters");

    let read_data = read_from_vault(password.clone(), namespace.clone())
        .await
        .expect("Failed to read vault with special characters");

    assert_eq!(read_data.as_string().unwrap(), "test_data");

    remove_from_vault(password, namespace)
        .await
        .expect("Failed to remove vault with special characters");
}

#[wasm_bindgen_test]
async fn test_concurrent_vault_operations() {
    cleanup_all_vaults().await;

    let passwords: Vec<JsValue> = (0..3).map(|i| format!("password{}", i).into()).collect();
    let namespaces: Vec<JsValue> = (0..3).map(|i| format!("namespace{}", i).into()).collect();
    let data: Vec<JsValue> = (0..3).map(|i| format!("data{}", i).into()).collect();

    let mut create_futures = Vec::new();
    for i in 0..3 {
        let future = create_vault(
            passwords[i].clone(),
            namespaces[i].clone(),
            data[i].clone(),
        );
        create_futures.push(future);
    }

    for future in create_futures {
        future.await.expect("Failed to create vault in concurrent operation");
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_empty_namespace() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "".into();
    let data: JsValue = "test_data".into();

    let result = create_vault(password.clone(), namespace.clone(), data.clone()).await;
    assert!(result.is_err(), "Should fail with empty namespace");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_empty_data() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "".into();

    create_vault(password.clone(), namespace.clone(), data.clone()).await.expect("Failed to create vault with empty data");

    let read_data = read_from_vault(password.clone(), namespace.clone()).await.expect("Failed to read from vault with empty data");
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
        let result = create_vault_with_name(invalid_name.clone(), password.clone(), namespace.clone(), data.clone()).await;
        assert!(
            result.is_err(),
            "Should fail with invalid vault name: '{}', but succeeded",
            name
        );
        
        if let Ok(_) = result {
            let _ = remove_vault_with_name(name).await;
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

    let mut create_futures = Vec::new();
    for i in 0..3 {
        let future = create_vault_with_name(
            vault_names[i].clone(),
            passwords[i].clone(),
            namespaces[i].clone(),
            data[i].clone(),
        );
        create_futures.push(future);
    }

    for future in create_futures {
        future.await.expect("Failed to create vault in concurrent operation");
    }

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_read_non_existent_namespace() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "non_existent_namespace".into();

    let result = read_from_vault(password, namespace).await;
    assert!(result.is_err(), "Reading from a non-existent namespace should fail");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_list_namespaces_in_empty_vault() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "initial_namespace".into();
    let data: JsValue = "initial_data".into();

    create_vault(password.clone(), namespace.clone(), data.clone()).await.expect("Failed to create initial vault");

    remove_from_vault(password.clone(), namespace.clone()).await.expect("Failed to remove initial namespace");

    let listed = list_namespaces(password).await.expect("Failed to list namespaces");
    let listed_namespaces: Vec<String> = from_value(listed).expect("Failed to convert namespaces");

    assert!(listed_namespaces.is_empty(), "Namespaces list should be empty");

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_large_data_payload() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let large_data: JsValue = "a".repeat(10_000_000).into(); // 10 MB of data

    create_vault(password.clone(), namespace.clone(), large_data.clone()).await.expect("Failed to create vault with large data");

    let read_data = read_from_vault(password.clone(), namespace.clone()).await.expect("Failed to read from vault with large data");
    assert_eq!(read_data.as_string().unwrap(), "a".repeat(10_000_000));

    cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_concurrent_read_operations() {
    cleanup_all_vaults().await;

    let password: JsValue = "test_password123".into();
    let namespace: JsValue = "test_namespace".into();
    let data: JsValue = "test_data".into();

    create_vault(password.clone(), namespace.clone(), data.clone()).await.expect("Failed to create vault");

    let mut read_futures = Vec::new();
    for _ in 0..3 {
        let future = read_from_vault(password.clone(), namespace.clone());
        read_futures.push(future);
    }

    for future in read_futures {
        let read_data = future.await.expect("Failed to read from vault concurrently");
        assert_eq!(read_data.as_string().unwrap(), "test_data");
    }

    cleanup_all_vaults().await;
}
