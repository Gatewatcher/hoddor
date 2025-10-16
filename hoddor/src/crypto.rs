use age::{
    secrecy::ExposeSecret,
    x25519::{Identity, Recipient},
};
use crate::domain::crypto;
use crate::platform::Platform;
use js_sys::Uint8Array;
use rand::{thread_rng, Rng};
use std::fmt;
use wasm_bindgen::prelude::*;
use web_sys::AuthenticationExtensionsPrfValues;

pub fn gen_random() -> [u8; 32] {
    thread_rng().gen::<[u8; 32]>()
}

#[wasm_bindgen]
pub async fn identity_from_passphrase(
    passphrase: &str,
    salt: &[u8],
) -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();
    identity_from_passphrase_internal(&platform, passphrase, salt).await
}

async fn identity_from_passphrase_internal(
    platform: &Platform,
    passphrase: &str,
    salt: &[u8],
) -> Result<IdentityHandle, JsValue> {
    let identity_str = crypto::identity_from_passphrase(platform, passphrase, salt)
        .await
        .map_err(|e| {
            platform
                .logger()
                .log(&format!("Failed to derive identity: {}", e));
            JsValue::from_str(&e.to_string())
        })?;

    let identity: Identity = identity_str
        .parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    Ok(IdentityHandle::from(identity))
}

/// Generate a new Age identity (key pair)
#[wasm_bindgen]
pub fn generate_identity() -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();

    let identity_str = crypto::generate_identity(&platform)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let identity: Identity = identity_str
        .parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    Ok(IdentityHandle::from(identity))
}

/// Parse a recipient string into an Age recipient
#[wasm_bindgen]
pub fn parse_recipient(recipient: &str) -> Result<RecipientHandle, JsValue> {
    let platform = Platform::new();

    crypto::parse_recipient(&platform, recipient)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // If validation passed, parse the recipient
    recipient
        .parse::<Recipient>()
        .map(Into::into)
        .map_err(|e| JsValue::from_str(&format!("Invalid recipient: {}", e)))
}

/// Encrypt data with recipients (public keys)
pub async fn encrypt_with_recipients(
    data: &[u8],
    recipients: &[RecipientHandle],
) -> Result<Vec<u8>, JsValue> {
    let platform = Platform::new();
    let recipient_strs: Vec<String> = recipients.iter().map(|r| r.to_string()).collect();
    let recipient_refs: Vec<&str> = recipient_strs.iter().map(|s| s.as_str()).collect();

    crypto::encrypt_for_recipients(&platform, data, &recipient_refs)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Decrypt data with an identity (private key)
pub async fn decrypt_with_identity(
    encrypted_data: &[u8],
    identity_handle: &IdentityHandle,
) -> Result<Vec<u8>, JsValue> {
    let platform = Platform::new();
    let identity_str = identity_handle.private_key();

    crypto::decrypt_with_identity(&platform, encrypted_data, &identity_str)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub struct RecipientHandle {
    recipient: Recipient,
}

impl fmt::Debug for RecipientHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecipientHandle")
            .field("public_key", &self.recipient.to_string())
            .finish()
    }
}

#[wasm_bindgen]
impl RecipientHandle {
    pub fn to_string(&self) -> String {
        self.recipient.to_string()
    }
}

impl From<Recipient> for RecipientHandle {
    fn from(recipient: Recipient) -> Self {
        Self { recipient }
    }
}

impl AsRef<Recipient> for RecipientHandle {
    fn as_ref(&self) -> &Recipient {
        &self.recipient
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct IdentityHandle {
    identity: Identity,
}

impl fmt::Debug for IdentityHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IdentityHandle {{ public_key: {} }}",
            self.identity.to_public()
        )
    }
}

impl fmt::Display for IdentityHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identity.to_public())
    }
}

impl AsRef<dyn age::Identity + 'static> for IdentityHandle {
    fn as_ref(&self) -> &(dyn age::Identity + 'static) {
        &self.identity
    }
}

#[wasm_bindgen]
impl IdentityHandle {
    pub fn public_key(&self) -> String {
        self.identity.to_public().to_string()
    }

    pub fn to_public(&self) -> RecipientHandle {
        RecipientHandle::from(self.identity.to_public())
    }

    pub fn private_key(&self) -> String {
        self.identity.to_string().expose_secret().to_string()
    }

    pub fn to_json(&self) -> JsValue {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"public_key".into(), &self.public_key().into()).unwrap();
        js_sys::Reflect::set(&obj, &"private_key".into(), &self.private_key().into()).unwrap();
        obj.into()
    }

    pub fn from_json(json: &JsValue) -> Result<IdentityHandle, JsValue> {
        let private_key = js_sys::Reflect::get(json, &"private_key".into())
            .map_err(|_| JsValue::from_str("Missing private_key field"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("private_key must be a string"))?;

        let identity = private_key
            .parse::<Identity>()
            .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

        Ok(IdentityHandle::from(identity))
    }
}

impl From<Identity> for IdentityHandle {
    fn from(identity: Identity) -> Self {
        IdentityHandle { identity }
    }
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
