use crate::{global::window, measure::time_it};
use argon2::Argon2;
use hkdf::Hkdf;
use js_sys::{Array, Uint8Array};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::JsValue;
use web_sys::AuthenticationExtensionsPrfValues;

pub fn gen_random<const N: usize>() -> Result<[u8; N], JsValue> {
    let mut result: [u8; N] = [0; N];
    window()
        .crypto()?
        .get_random_values_with_u8_array(&mut result)?;
    Ok(result)
}

pub fn derive_key(password: &[u8], salt: &[u8]) -> Result<[u8; 32], JsValue> {
    time_it!("derive_key", {
        let argon2 = Argon2::default();
        let mut key = [0u8; 32];

        argon2
            .hash_password_into(password, salt, &mut key)
            .map_err(|e| JsValue::from_str(&format!("Key derivation failed: {:?}", e)))?;

        Ok(key)
    })
}

pub fn prf_inputs(nonce: &Uint8Array) -> AuthenticationExtensionsPrfValues {
    let prefix = "hoddor/prf".as_bytes().to_vec();

    let mut first = prefix.clone();
    first.push(0x01);
    first.extend(nonce.to_vec());

    let mut second = prefix;
    second.push(0x02);
    second.extend(nonce.to_vec());

    let prf_inputs =
        AuthenticationExtensionsPrfValues::new(&Uint8Array::new(&JsValue::from(first)));
    prf_inputs.set_second(&Uint8Array::new(&JsValue::from(second)));
    prf_inputs
}

pub fn derive_key_from_outputs(
    prf_outputs: AuthenticationExtensionsPrfValues,
) -> [u8; 32] {
    let first = Uint8Array::new(&prf_outputs.get_first());
    let second = match prf_outputs.get_second() {
        None => Uint8Array::new(&Array::new()),
        Some(second) => Uint8Array::new(&second),
    };

    let mut prf = first.to_vec();
    prf.extend(second.to_vec());

    let mixed_prf = Sha256::digest(&prf);

    let (prk, _) = Hkdf::<Sha256>::extract(Some("hoddor/vault".as_bytes()), mixed_prf.as_slice());
    prk.into()
}
