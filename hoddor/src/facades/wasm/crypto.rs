use super::converters;
use crate::domain::crypto;
use crate::platform::Platform;
use age::{
    secrecy::ExposeSecret,
    x25519::{Identity, Recipient},
};
use std::fmt;
use wasm_bindgen::prelude::*;

/// Generate a new Age identity (key pair)
#[wasm_bindgen]
pub fn generate_identity() -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();

    let identity_str = crypto::generate_identity(&platform).map_err(converters::to_js_error)?;

    let identity: Identity = identity_str
        .parse()
        .map_err(|e| converters::to_js_error(format!("Failed to parse identity: {}", e)))?;

    Ok(IdentityHandle::from(identity))
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
            .map_err(|e| converters::to_js_error(format!("Failed to parse identity: {}", e)))?;

        Ok(IdentityHandle::from(identity))
    }
}

impl From<Identity> for IdentityHandle {
    fn from(identity: Identity) -> Self {
        IdentityHandle { identity }
    }
}
