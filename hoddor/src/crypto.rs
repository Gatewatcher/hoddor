use age::{
    secrecy::ExposeSecret,
    x25519::{Identity, Recipient},
    Decryptor, Encryptor,
};
use argon2::Argon2;
use hkdf::Hkdf;
use js_sys::Uint8Array;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::JsValue;
use web_sys::AuthenticationExtensionsPrfValues;

pub fn gen_random() -> [u8; 32] {
    thread_rng().gen::<[u8; 32]>()
}
use crate::console;
use bech32::{ToBase32, Variant};
use futures::io::{AllowStdIo, AsyncReadExt, AsyncWriteExt};
use std::fmt;
use std::io::Cursor;
use wasm_bindgen::prelude::*;
use x25519_dalek::StaticSecret;
use zeroize::Zeroize;

#[wasm_bindgen]
pub async fn identity_from_passphrase(
    passphrase: &str,
    salt: &[u8],
) -> Result<IdentityHandle, JsValue> {
    // Use Argon2 to derive a stable seed from the passphrase
    let argon2 = Argon2::default();
    let mut seed = [0u8; 32];

    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut seed)
        .map_err(|e| JsValue::from_str(&format!("Key derivation failed: {:?}", e)))?;

    // Create a static secret from the derived key
    let secret = StaticSecret::from(seed);

    // Create a secret string in the format Age expects
    let mut sk_bytes = secret.to_bytes();
    let sk_base32 = sk_bytes.to_base32();
    let encoded = bech32::encode("age-secret-key-", sk_base32, Variant::Bech32)
        .map_err(|e| JsValue::from_str(&format!("Failed to encode identity: {}", e)))?
        .to_uppercase();

    // Clear sensitive data
    sk_bytes.zeroize();
    seed.zeroize();

    // Parse into Age identity
    let identity = encoded.parse::<Identity>().map_err(|e| {
        console::log(&format!("Failed to parse identity string: {}", encoded));
        JsValue::from_str(&format!("Failed to create identity: {}", e))
    })?;

    Ok(IdentityHandle::from(identity))
}

/// Generate a new Age identity (key pair)
#[wasm_bindgen]
pub fn generate_identity() -> Result<IdentityHandle, JsValue> {
    let identity = Identity::generate();

    Ok(IdentityHandle::from(identity))
}

/// Parse a recipient string into an Age recipient
#[wasm_bindgen]
pub fn parse_recipient(recipient: &str) -> Result<RecipientHandle, JsValue> {
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
    let recipients: Vec<_> = recipients.iter().map(|r| r.as_ref()).collect();
    let encryptor = Encryptor::with_recipients(
        recipients
            .iter()
            .map(|r| Box::new((*r).clone()) as Box<dyn age::Recipient + Send>)
            .collect(),
    )
    .ok_or_else(|| JsValue::from_str("No recipients provided"))?;

    let mut encrypted = vec![];
    let cursor = Cursor::new(&mut encrypted);
    let async_cursor = AllowStdIo::new(cursor);
    let mut writer = encryptor
        .wrap_output(Box::new(async_cursor))
        .map_err(|e| JsValue::from_str(&format!("Failed to initialize encryption: {}", e)))?;

    AsyncWriteExt::write_all(&mut writer, data)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to encrypt data: {}", e)))?;

    AsyncWriteExt::close(&mut writer)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to finalize encryption: {}", e)))?;

    Ok(encrypted)
}

/// Decrypt data with an identity (private key)
pub async fn decrypt_with_identity(
    encrypted_data: &[u8],
    identity_handle: &IdentityHandle,
) -> Result<Vec<u8>, JsValue> {
    let decryptor = match Decryptor::new(encrypted_data) {
        Ok(d) => d,
        Err(e) => {
            return Err(JsValue::from_str(&format!(
                "Failed to create decryptor: {}",
                e
            )));
        }
    };

    match decryptor {
        Decryptor::Recipients(d) => {
            let mut decrypted = vec![];
            let reader = d
                .decrypt(std::iter::once(identity_handle.as_ref()))
                .map_err(|e| JsValue::from_str(&format!("Failed to decrypt: {}", e)))?;

            let mut async_reader = AllowStdIo::new(reader);
            AsyncReadExt::read_to_end(&mut async_reader, &mut decrypted)
                .await
                .map_err(|e| JsValue::from_str(&format!("Failed to read decrypted data: {}", e)))?;

            Ok(decrypted)
        }
        _ => Err(JsValue::from_str("File was not encrypted with recipients")),
    }
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

fn validate_prf_outputs(prf_output: &AuthenticationExtensionsPrfValues) -> Result<(), JsValue> {
    if prf_output.get_first().is_undefined() {
        return Err(JsValue::from_str("Missing first PRF value"));
    }
    if prf_output.get_second().is_none() {
        return Err(JsValue::from_str("Missing second PRF value"));
    }
    Ok(())
}

pub fn derive_key_from_outputs(
    prf_outputs: &AuthenticationExtensionsPrfValues,
) -> Result<[u8; 32], JsValue> {
    validate_prf_outputs(prf_outputs)?;

    let first = Uint8Array::new(&prf_outputs.get_first());
    let second = Uint8Array::new(&prf_outputs.get_second().unwrap());

    let mut prf = first.to_vec();
    prf.extend(second.to_vec());

    let mixed_prf = Sha256::digest(&prf);

    let (prk, _) = Hkdf::<Sha256>::extract(Some("hoddor/vault".as_bytes()), mixed_prf.as_slice());
    Ok(prk.into())
}

pub fn identity_from_prf(
    prf_output: &web_sys::AuthenticationExtensionsPrfValues,
) -> Result<IdentityHandle, JsValue> {
    validate_prf_outputs(prf_output)?;

    let seed = derive_key_from_outputs(prf_output)?;
    if seed.iter().all(|&x| x == 0) {
        return Err(JsValue::from_str("Invalid PRF seed (all zeros)"));
    }

    let secret = StaticSecret::from(seed);
    let mut sk_bytes = secret.to_bytes();

    // Validate key bytes
    if sk_bytes.iter().all(|&x| x == 0) {
        sk_bytes.zeroize();
        return Err(JsValue::from_str("Generated invalid secret key"));
    }

    let sk_base32 = sk_bytes.to_base32();
    let encoded = bech32::encode("age-secret-key-", sk_base32, Variant::Bech32)
        .map_err(|e| {
            sk_bytes.zeroize();
            JsValue::from_str(&format!("Failed to encode identity: {}", e))
        })?
        .to_uppercase();

    sk_bytes.zeroize();

    let identity = encoded.parse::<Identity>().map_err(|e| {
        console::log(&format!("Failed to parse identity string: {}", encoded));
        JsValue::from_str(&format!("Failed to create identity: {}", e))
    })?;

    let handle = IdentityHandle {
        identity: identity.clone(),
    };

    // Validate the created handle
    if handle.public_key().is_empty() || handle.private_key().is_empty() {
        return Err(JsValue::from_str("Generated invalid identity handle"));
    }

    Ok(handle)
}
