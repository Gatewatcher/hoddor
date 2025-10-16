use age::x25519::Identity;
use crate::crypto::IdentityHandle;
use crate::domain::crypto;
use crate::platform::Platform;
use js_sys::Uint8Array;
use rand::{thread_rng, Rng};
use wasm_bindgen::prelude::*;
use web_sys::AuthenticationExtensionsPrfValues;

/// Generate random 32 bytes for WebAuthn operations
pub fn gen_random() -> [u8; 32] {
    thread_rng().gen::<[u8; 32]>()
}

/// Create PRF inputs from a nonce for WebAuthn
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

/// Extract PRF outputs from WebAuthn response
fn prf_outputs_from_js(
    prf: &AuthenticationExtensionsPrfValues,
) -> Result<(Vec<u8>, Vec<u8>), JsValue> {
    let first = if !prf.get_first().is_undefined() {
        Uint8Array::new(&prf.get_first()).to_vec()
    } else {
        return Err(JsValue::from_str("Missing first PRF value"));
    };

    let second = if let Some(s) = prf.get_second() {
        Uint8Array::new(&s).to_vec()
    } else {
        return Err(JsValue::from_str("Missing second PRF value"));
    };

    Ok((first, second))
}

/// Derive an identity from WebAuthn PRF outputs
pub fn identity_from_prf(
    prf_output: &web_sys::AuthenticationExtensionsPrfValues,
) -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();
    identity_from_prf_internal(&platform, prf_output)
}

fn identity_from_prf_internal(
    platform: &Platform,
    prf_output: &web_sys::AuthenticationExtensionsPrfValues,
) -> Result<IdentityHandle, JsValue> {
    let (first, second) = prf_outputs_from_js(prf_output)?;

    let identity_str = crypto::identity_from_prf(platform, &first, &second).map_err(|e| {
        platform
            .logger()
            .log(&format!("Failed to derive identity from PRF: {}", e));
        JsValue::from_str(&e.to_string())
    })?;

    let identity: Identity = identity_str
        .parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    let handle = IdentityHandle::from(identity);

    // Validate handle
    if handle.public_key().is_empty() || handle.private_key().is_empty() {
        return Err(JsValue::from_str("Generated invalid identity handle"));
    }

    Ok(handle)
}
