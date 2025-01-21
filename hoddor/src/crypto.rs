use age::{
    secrecy::{ExposeSecret, SecretString},
    x25519::{Identity, Recipient},
    Decryptor, Encryptor,
};
use argon2::Argon2;
use hkdf::Hkdf;
use js_sys::{Array, Uint8Array};
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::JsValue;
use web_sys::AuthenticationExtensionsPrfValues;

pub fn gen_random() -> [u8; 32] {
    thread_rng().gen::<[u8; 32]>()
}
use bech32::{ToBase32, FromBase32, Variant};
use chacha20::{
    cipher::{KeyIvInit, StreamCipher},
    ChaCha20,
};
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, AllowStdIo};
use rand_core::{CryptoRng, Error as RngError, RngCore};
use std::cell::RefCell;
use std::io::{Cursor, Read};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use x25519_dalek::{StaticSecret, PublicKey};
use std::fmt;
use crate::console;
use zeroize::Zeroize;

/// A deterministic CSPRNG using ChaCha20
struct DeterministicRng {
    cipher: RefCell<ChaCha20>,
    buffer: RefCell<[u8; 64]>,  // Buffer for generated random bytes
    buffer_pos: RefCell<usize>, // Current position in buffer
}

impl DeterministicRng {
    fn new(seed: &[u8; 32]) -> Self {
        // Use first 32 bytes as key, and a fixed nonce
        let nonce = [0u8; 12]; // ChaCha20 uses 12-byte nonces
        let cipher = ChaCha20::new(seed.into(), &nonce.into());
        
        Self {
            cipher: RefCell::new(cipher),
            buffer: RefCell::new([0u8; 64]),
            buffer_pos: RefCell::new(64), // Start at 64 to force initial refill
        }
    }

    fn refill_buffer(&self) {
        // Get a new buffer of zeros
        let mut buffer = [0u8; 64];
        
        // Encrypt zeros to get random bytes
        self.cipher.borrow_mut().apply_keystream(&mut buffer);
        
        // Update the internal buffer
        *self.buffer.borrow_mut() = buffer;
        
        // Reset position
        *self.buffer_pos.borrow_mut() = 0;
    }
}

impl RngCore for DeterministicRng {
    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0u8; 4];
        self.fill_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }

    fn next_u64(&mut self) -> u64 {
        let mut bytes = [0u8; 8];
        self.fill_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut remaining = dest;
        
        while !remaining.is_empty() {
            let pos = *self.buffer_pos.borrow();
            
            if pos >= 64 {
                self.refill_buffer();
            }
            
            let buffer = self.buffer.borrow();
            let pos = *self.buffer_pos.borrow(); // Get position again after potential refill
            let available = 64 - pos;
            let to_copy = remaining.len().min(available);
            
            remaining[..to_copy].copy_from_slice(&buffer[pos..pos + to_copy]);
            *self.buffer_pos.borrow_mut() = pos + to_copy;
            remaining = &mut remaining[to_copy..];
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), RngError> {
        self.fill_bytes(dest);
        Ok(())
    }
}

// Implement CryptoRng to mark this as cryptographically secure
impl CryptoRng for DeterministicRng {}

// Generate an Age identity from a passphrase and salt
#[wasm_bindgen]
pub async fn identity_from_passphrase(passphrase: &str, salt: &[u8]) -> Result<IdentityHandle, JsValue> {
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
    encoded
        .parse::<Identity>()
        .map(Into::into)
        .map_err(|e| {
            console::log(&format!("Failed to parse identity string: {}", encoded));
            JsValue::from_str(&format!("Failed to create identity: {}", e))
        })
}

/// Generate a new Age identity (key pair)
#[wasm_bindgen]
pub fn generate_identity() -> Result<IdentityHandle, JsValue> {
    Ok(Identity::generate().into())
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
pub async fn encrypt_with_recipients(data: &[u8], recipients: &[RecipientHandle]) -> Result<Vec<u8>, JsValue> {
    let recipients: Vec<_> = recipients.iter().map(|r| r.as_ref()).collect();
    let encryptor = Encryptor::with_recipients(recipients.iter().map(|r| Box::new((*r).clone()) as Box<dyn age::Recipient + Send>).collect())
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
pub async fn decrypt_with_identity(encrypted_data: &[u8], identity_handle: &IdentityHandle) -> Result<Vec<u8>, JsValue> {
    let cursor = Cursor::new(encrypted_data);
    let decryptor = match Decryptor::new(cursor) {
        Ok(d) => d,
        Err(e) => return Err(JsValue::from_str(&format!("Failed to create decryptor: {}", e))),
    };

    match decryptor {
        Decryptor::Recipients(d) => {
            let mut decrypted = vec![];
            let reader = d
                .decrypt(std::iter::once(identity_handle.as_identity() as &dyn age::Identity))
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
pub struct IdentityHandle {
    identity: Identity,
}

impl fmt::Debug for IdentityHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentityHandle")
            .field("public_key", &self.identity.to_public().to_string())
            .finish()
    }
}

impl fmt::Display for IdentityHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identity.to_public().to_string())
    }
}

#[wasm_bindgen]
impl IdentityHandle {
    #[wasm_bindgen(getter)]
    pub fn public_key(&self) -> String {
        self.identity.to_public().to_string()
    }

    pub fn to_public(&self) -> RecipientHandle {
        RecipientHandle::from(self.identity.to_public())
    }

    pub(crate) fn as_identity(&self) -> &Identity {
        &self.identity
    }

    #[wasm_bindgen(getter)]
    pub fn private_key(&self) -> String {
        self.identity.to_string().expose_secret().to_string()
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        // Convert the identity to a string representation
        let identity_str = self.identity.to_string().expose_secret().to_string();
        
        // Create a JS object with the serialized identity
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"identity".into(), &identity_str.into()).unwrap();
        obj.into()
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(json: &JsValue) -> Result<IdentityHandle, JsValue> {
        // Get the identity string from the JS object
        let identity_str = js_sys::Reflect::get(json, &"identity".into())
            .map_err(|e| format!("Failed to get identity from JSON: {:?}", e))?
            .as_string()
            .ok_or("Identity is not a string")?;
        
        // Parse the identity string back into an Age identity
        identity_str
            .parse::<Identity>()
            .map(Into::into)
            .map_err(|e| format!("Failed to parse identity: {}", e).into())
    }
}

impl From<Identity> for IdentityHandle {
    fn from(identity: Identity) -> Self {
        Self { identity }
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

pub fn derive_key_from_outputs(prf_outputs: AuthenticationExtensionsPrfValues) -> [u8; 32] {
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
