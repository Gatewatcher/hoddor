#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use hoddor::{
    facades::wasm::vault::{
        create_vault, remove_from_vault, upsert_vault, vault_identity_from_passphrase,
    },
    platform::Platform,
};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

mod test_utils;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn addEventListener(event: &str, callback: &Closure<dyn FnMut(web_sys::MessageEvent)>);

    #[wasm_bindgen(js_namespace = window)]
    fn removeEventListener(event: &str, callback: &Closure<dyn FnMut(web_sys::MessageEvent)>);
}

struct MessageListener {
    messages: Rc<RefCell<Vec<JsValue>>>,
    closure: Closure<dyn FnMut(web_sys::MessageEvent)>,
}

impl MessageListener {
    fn new() -> Self {
        let messages = Rc::new(RefCell::new(Vec::new()));
        let messages_clone = messages.clone();

        let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let data = event.data();
            Platform::new()
                .logger()
                .log(&format!("MessageListener captured: {:?}", data));
            messages_clone.borrow_mut().push(data);
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        let window = web_sys::window().expect("no window");
        window
            .add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())
            .expect("failed to add event listener");

        Self { messages, closure }
    }

    fn get_messages(&self) -> Vec<JsValue> {
        self.messages.borrow().clone()
    }

    fn clear(&self) {
        self.messages.borrow_mut().clear();
    }

    fn wait_for_message(&self, timeout_ms: u32) -> Option<JsValue> {
        let start = js_sys::Date::now();
        loop {
            if !self.messages.borrow().is_empty() {
                return self.messages.borrow_mut().pop();
            }
            if js_sys::Date::now() - start > timeout_ms as f64 {
                return None;
            }
            std::hint::spin_loop();
        }
    }
}

impl Drop for MessageListener {
    fn drop(&mut self) {
        let window = web_sys::window().expect("no window");
        let _ = window
            .remove_event_listener_with_callback("message", self.closure.as_ref().unchecked_ref());
    }
}

#[wasm_bindgen_test]
async fn test_notification_on_upsert() {
    test_utils::cleanup_all_vaults().await;

    let listener = MessageListener::new();
    listener.clear();

    let vault_name = "notification_test";
    let password = "test_password";
    let namespace = "test_namespace";
    let data: JsValue = "test_data".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    if let Some(message) = listener.wait_for_message(1000) {
        Platform::new().logger().log("Got notification message!");

        let event_type = js_sys::Reflect::get(&message, &JsValue::from_str("event"))
            .ok()
            .and_then(|v| v.as_string());

        if let Some(event) = event_type {
            assert_eq!(event, "vaultUpdate", "Expected vaultUpdate event");

            let has_data = js_sys::Reflect::has(&message, &JsValue::from_str("data"))
                .expect("Failed to check data field");
            assert!(has_data, "Notification should contain data field");

            Platform::new().logger().log("✅ Notification test passed!");
        } else {
            panic!("Message doesn't have an event field");
        }
    } else {
        panic!("No notification received within timeout");
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_notification_contains_vault_data() {
    test_utils::cleanup_all_vaults().await;

    let listener = MessageListener::new();
    listener.clear();

    let vault_name = "data_notification_test";
    let password = "test_password";
    let namespace = "test_namespace";
    let data: JsValue = "test_data".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    if let Some(message) = listener.wait_for_message(1000) {
        let vault_data = js_sys::Reflect::get(&message, &JsValue::from_str("data"))
            .expect("Failed to get data field");

        assert!(vault_data.is_object(), "Data should be an object");

        let has_metadata = js_sys::Reflect::has(&vault_data, &JsValue::from_str("metadata"))
            .expect("Failed to check metadata");
        assert!(has_metadata, "Vault data should contain metadata");

        let has_namespaces = js_sys::Reflect::has(&vault_data, &JsValue::from_str("namespaces"))
            .expect("Failed to check namespaces");
        assert!(has_namespaces, "Vault data should contain namespaces");

        Platform::new()
            .logger()
            .log("✅ Notification data structure test passed!");
    } else {
        panic!("No notification received within timeout");
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_notification_on_remove() {
    test_utils::cleanup_all_vaults().await;

    let listener = MessageListener::new();

    let vault_name = "remove_notification_test";
    let password = "test_password";
    let namespace = "test_namespace";
    let data: JsValue = "test_data".into();

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    upsert_vault(vault_name, &identity, namespace, data.clone(), None, false)
        .await
        .expect("Failed to upsert data");

    listener.clear();

    remove_from_vault(vault_name, &identity, JsValue::from_str(namespace))
        .await
        .expect("Failed to remove namespace");

    if let Some(message) = listener.wait_for_message(1000) {
        let event_type = js_sys::Reflect::get(&message, &JsValue::from_str("event"))
            .ok()
            .and_then(|v| v.as_string());

        assert_eq!(
            event_type,
            Some("vaultUpdate".to_string()),
            "Expected vaultUpdate event on remove"
        );

        Platform::new()
            .logger()
            .log("✅ Remove notification test passed!");
    } else {
        panic!("No notification received after remove operation");
    }

    test_utils::cleanup_all_vaults().await;
}

#[wasm_bindgen_test]
async fn test_multiple_notifications() {
    test_utils::cleanup_all_vaults().await;

    let listener = MessageListener::new();
    listener.clear();

    let vault_name = "multi_notification_test";
    let password = "test_password";

    create_vault(JsValue::from_str(vault_name))
        .await
        .expect("Failed to create vault");

    let identity = vault_identity_from_passphrase(password, vault_name)
        .await
        .expect("Failed to create identity");

    for i in 0..3 {
        let namespace = format!("namespace_{}", i);
        let data: JsValue = format!("data_{}", i).into();

        upsert_vault(vault_name, &identity, &namespace, data.clone(), None, false)
            .await
            .expect("Failed to upsert data");
    }

    gloo_timers::future::TimeoutFuture::new(500).await;

    let messages = listener.get_messages();

    Platform::new().logger().log(&format!(
        "Received {} notification messages",
        messages.len()
    ));

    assert!(
        messages.len() >= 3,
        "Should have received at least 3 notifications, got {}",
        messages.len()
    );

    for (i, message) in messages.iter().enumerate() {
        let event_type = js_sys::Reflect::get(message, &JsValue::from_str("event"))
            .ok()
            .and_then(|v| v.as_string());

        assert_eq!(
            event_type,
            Some("vaultUpdate".to_string()),
            "Message {} should be vaultUpdate",
            i
        );
    }

    Platform::new()
        .logger()
        .log("✅ Multiple notifications test passed!");

    test_utils::cleanup_all_vaults().await;
}
