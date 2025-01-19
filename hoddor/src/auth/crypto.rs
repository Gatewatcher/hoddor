use js_sys::Array;
use js_sys::Uint8Array;
use sha2::{Digest, Sha256};
use wasm_bindgen::JsValue;
use web_sys::AuthenticationExtensionsPrfValues;

use crate::crypto::derive_key;

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
) -> Result<[u8; 32], JsValue> {
    let first = Uint8Array::new(&prf_outputs.get_first());
    let second = match prf_outputs.get_second() {
        None => Uint8Array::new(&Array::new()),
        Some(second) => Uint8Array::new(&second),
    };

    let mut prf = first.to_vec();
    prf.extend(second.to_vec());

    let mixed_prf = Sha256::digest(&prf);

    derive_key(mixed_prf.as_slice(), "hoddor/vault".as_bytes())
}
