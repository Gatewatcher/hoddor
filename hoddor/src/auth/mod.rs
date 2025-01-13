use js_sys::Uint8Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Crypto, PublicKeyCredential, PublicKeyCredentialRpEntity, PublicKeyCredentialUserEntity,
};
use webauthn::{webauthn_create, webauthn_get};

use crate::console::*;
mod webauthn;

pub fn crypto() -> Result<Crypto, JsValue> {
    web_sys::window().ok_or(JsValue::UNDEFINED)?.crypto()
}

pub fn gen_random<const N: usize>() -> Result<[u8; N], JsValue> {
    let mut result: [u8; N] = [0; N];
    crypto()?.get_random_values_with_u8_array(&mut result)?;
    Ok(result)
}

static UUID_TEST: &str = "dwadawadw.awdawd.awdawd.dawdaw";
static SALT_TEST: [u8; 32] = [
    167, 229, 117, 95, 216, 60, 55, 245, 101, 198, 174, 106, 171, 68, 8, 211, 69, 45, 61, 22, 46,
    121, 232, 219, 42, 246, 223, 109, 78, 30, 226, 56,
];

#[wasm_bindgen]
pub async fn create_credential(username: &str) -> Result<PublicKeyCredential, JsValue> {
    log(&format!("Init credential creation"));

    // let prf_salt: [u8; 32] = gen_random()?;
    let prf_salt: [u8; 32] = SALT_TEST;
    let challenge: [u8; 32] = gen_random()?;

    Ok(JsFuture::from(webauthn_create(
        challenge.as_slice(),
        PublicKeyCredentialUserEntity::new(
            username,
            username,
            &Uint8Array::from(UUID_TEST.as_bytes().to_vec().as_slice()),
        ),
        {
            let pk_rp_entity = PublicKeyCredentialRpEntity::new("Vault");
            pk_rp_entity.set_id("localhost");
            pk_rp_entity
        },
        prf_salt,
    )?)
    .await?
    .into())
}

#[wasm_bindgen]
pub async fn get_credential() -> Result<PublicKeyCredential, JsValue> {
    log(&format!("Init credential get"));

    // let prf_salt: [u8; 32] = gen_random()?;
    let prf_salt: [u8; 32] = SALT_TEST;
    let challenge: [u8; 32] = gen_random()?;

    Ok(JsFuture::from(webauthn_get(
        challenge.as_slice(),
        UUID_TEST,
        "localhost",
        prf_salt,
    )?)
    .await?
    .into())
}
