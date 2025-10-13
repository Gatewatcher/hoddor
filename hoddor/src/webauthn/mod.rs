use crate::crypto::IdentityHandle;
use js_sys::Uint8Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AuthenticationExtensionsPrfValues, PublicKeyCredential};
use webauthn::{webauthn_create, webauthn_get};

use crate::vault::{get_vault, save_vault};
use crate::{
    adapters::logger,
    crypto::{gen_random, identity_from_prf},

};
use rand::rngs::OsRng;
use rand::RngCore;
pub mod webauthn;

fn get_identity_from_vault() -> Result<[u8; 32], JsValue> {
    let mut new_salt = [0u8; 32];
    OsRng.fill_bytes(&mut new_salt);
    Ok(new_salt)
}
#[wasm_bindgen]
pub async fn create_credential(
    vault_name: &str,
    username: &str,
) -> Result<IdentityHandle, JsValue> {
    logger().log(&"Init credential creation".to_string());

    let challenge = Uint8Array::from(gen_random().as_slice());
    let new_salt = get_identity_from_vault()?;
    let salt_array = Uint8Array::from(new_salt.as_slice());

    let (dir_handle, mut vault) = get_vault(vault_name).await?;

    let credential = JsFuture::from(webauthn_create(&challenge, username, &salt_array)?)
        .await?
        .dyn_into::<PublicKeyCredential>()
        .map_err(|_| JsValue::from_str("Failed to get credential"))?;

    // Extract PRF values from the authenticator response
    let extensions = credential.get_client_extension_results();

    let prf_results = js_sys::Reflect::get(&extensions, &"prf".into())
        .map_err(|_| JsValue::from_str("PRF extension not found"))?;

    let results = js_sys::Reflect::get(&prf_results, &"results".into())
        .map_err(|_| JsValue::from_str("PRF results.results not found"))?;

    let first = js_sys::Reflect::get(&results, &"first".into())
        .map_err(|_| JsValue::from_str("First PRF result not found"))?;

    let first: js_sys::ArrayBuffer = first
        .dyn_into()
        .map_err(|_| JsValue::from_str("First PRF result is not an ArrayBuffer"))?;

    let second = js_sys::Reflect::get(&results, &"second".into())
        .map_err(|_| JsValue::from_str("Second PRF result not found"))?;

    let second: js_sys::ArrayBuffer = second
        .dyn_into()
        .map_err(|_| JsValue::from_str("Second PRF result is not an ArrayBuffer"))?;

    let prf_values = AuthenticationExtensionsPrfValues::new(&Uint8Array::new(&first));
    prf_values.set_second(&Uint8Array::new(&second));
    
    let identity = identity_from_prf(&prf_values)?;
    let public_key = identity.public_key();

    vault.identity_salts.set_salt(public_key.clone(), new_salt);

    let raw_id = js_sys::Uint8Array::new(&credential.raw_id());
    let mut cred_id = vec![0; raw_id.length() as usize];
    raw_id.copy_to(&mut cred_id);
    vault
        .identity_salts
        .set_credential_id(public_key.clone(), cred_id.clone());
    
    vault
        .username_pk
        .insert(String::from(username), public_key.clone());

    save_vault(&dir_handle, vault)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to save vault: {:?}", e)))?;

    Ok(identity)
}

#[wasm_bindgen]
pub async fn get_credential(vault_name: &str, username: &str) -> Result<IdentityHandle, JsValue> {
    logger().log(&format!(
        "Init credential get for username: {}",
        username
    ));

    let challenge = Uint8Array::from(gen_random().as_slice());

    let (_, vault) = get_vault(vault_name).await?;

    let public_key = vault.username_pk.get(username).ok_or_else(|| {
        JsValue::from_str(&format!(
            "No public key found for username: {}",
            username
        ))
    })?;

    logger().log(&format!(
        "Found public key for username: {}, {:?}",
        username, public_key
    ));

    let credential_id = vault
        .identity_salts
        .get_credential_id(public_key)
        .ok_or_else(|| {
            JsValue::from_str(&format!(
                "No credential ID found for public key: {}",
                public_key
            ))
        })?;

    let salt = vault.identity_salts.get_salt(public_key).ok_or_else(|| {
        JsValue::from_str(&format!("No salt found for public key: {}", public_key))
    })?;

    logger().log(&format!(
        "Found credential ID and salt for public key: {}, {:?}",
        public_key, salt
    ));

    let credential = JsFuture::from(webauthn_get(
        &challenge,
        &Uint8Array::from(salt.as_slice()),
        Uint8Array::from(credential_id.as_slice()),
    )?)
    .await?
    .dyn_into::<PublicKeyCredential>()?;

    let extensions = credential.get_client_extension_results();

    let prf_results = js_sys::Reflect::get(&extensions, &"prf".into())
        .map_err(|_| JsValue::from_str("PRF extension not found"))?;

    let results = js_sys::Reflect::get(&prf_results, &"results".into())
        .map_err(|_| JsValue::from_str("PRF results.results not found"))?;
    let results: js_sys::Object = results
        .dyn_into()
        .map_err(|_| JsValue::from_str("Malformed PRF results.results"))?;

    let first = js_sys::Reflect::get(&results, &"first".into())
        .map_err(|_| JsValue::from_str("First PRF result not found"))?;
    logger().log(&format!("First value before conversion: {:?}", first));
    let first: js_sys::ArrayBuffer = first
        .dyn_into()
        .map_err(|_| JsValue::from_str("First PRF result is not an ArrayBuffer"))?;
    let first_array = Uint8Array::new(&first);
    logger().log(&format!(
        "First ArrayBuffer length: {}",
        first_array.length()
    ));
    if first_array.length() > 0 {
        let first_vec = first_array.to_vec();
        logger().log(&format!("First ArrayBuffer contents: {:?}", first_vec));
    }

    let second = js_sys::Reflect::get(&results, &"second".into())
        .ok()
        .and_then(|val| {
            logger().log(&format!("Second value before conversion: {:?}", val));
            let second_buf = val.dyn_into::<js_sys::ArrayBuffer>();
            if let Ok(buf) = second_buf {
                let second_array = Uint8Array::new(&buf);
                logger().log(&format!(
                    "Second ArrayBuffer length: {}",
                    second_array.length()
                ));
                if second_array.length() > 0 {
                    let second_vec = second_array.to_vec();
                    logger().log(&format!("Second ArrayBuffer contents: {:?}", second_vec));
                }
                Some(buf)
            } else {
                None
            }
        });

    let prf_values = AuthenticationExtensionsPrfValues::new(&Uint8Array::new(&first));
    if let Some(buf) = second {
        prf_values.set_second(&Uint8Array::new(&buf));
    }

    logger().log(&"PRF outputs processed successfully".to_string());

    let identity = identity_from_prf(&prf_values)?;

    if identity.public_key() != public_key.clone() {
        return Err(JsValue::from_str(&format!(
            "PRF-derived identity mismatch. Expected: {}, Got: {}",
            public_key,
            identity.public_key()
        )));
    }

    Ok(identity.clone())
}

#[wasm_bindgen]
pub async fn list_webauthn_public_keys(vault_name: &str) -> Result<JsValue, JsValue> {
    let (_, vault) = get_vault(vault_name).await?;

    let public_keys: Vec<String> = vault
        .identity_salts
        .get_public_keys_with_credentials()
        .map(|s| s.to_string())
        .collect();

    Ok(serde_wasm_bindgen::to_value(&public_keys)?)
}
