use js_sys::Uint8Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Crypto, PublicKeyCredential};
use webauthn::{webauthn_create, webauthn_get};

use crate::console::*;
pub mod webauthn;
pub mod crypto;

pub fn crypto() -> Result<Crypto, JsValue> {
    web_sys::window().ok_or(JsValue::UNDEFINED)?.crypto()
}

fn gen_random<const N: usize>() -> Result<[u8; N], JsValue> {
    let mut result: [u8; N] = [0; N];
    crypto()?.get_random_values_with_u8_array(&mut result)?;
    Ok(result)
}

#[wasm_bindgen]
pub async fn create_credential(
    name: &str,
    cred_id: Option<Uint8Array>,
) -> Result<PublicKeyCredential, JsValue> {
    log(&format!("Init credential creation"));

    let challenge = Uint8Array::from(gen_random::<32>()?.as_slice());

    let cred_id = match cred_id {
        None => Uint8Array::from(gen_random::<32>()?.as_slice()),
        Some(cred_id) => cred_id.clone(),
    };

    Ok(JsFuture::from(webauthn_create(&challenge, name, &cred_id)?)
        .await?
        .into())
}

#[wasm_bindgen]
pub async fn get_credential(
    prf_salt: &Uint8Array,
    cred_id: Option<Uint8Array>,
) -> Result<PublicKeyCredential, JsValue> {
    log(&format!("Init credential get"));

    let challenge = Uint8Array::from(gen_random::<32>()?.as_slice());

    Ok(JsFuture::from(webauthn_get(&challenge, prf_salt, cred_id)?)
        .await?
        .into())
}
