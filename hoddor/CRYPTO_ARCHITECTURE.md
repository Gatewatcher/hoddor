# Architecture Crypto - Ports & Adapters

## 🎯 Objectif

Extraire la logique cryptographique dans une architecture hexagonale (ports/adapters) pour permettre l'interopérabilité de Hoddor avec ou sans WASM (CLI, serveur, bibliothèque Rust native).

---

## 📊 Analyse Actuelle

### État de `crypto.rs` (350 lignes)

**Dépendances externes** (toutes compatibles WASM + native) :
- `age v0.10.1` - Chiffrement (types: Identity, Recipient)
- `argon2` - Dérivation de clé depuis passphrase
- `hkdf v0.12.4` - Dérivation HKDF pour PRF
- `sha2 v0.10.9` - Hash SHA-256
- `bech32 v0.9.1` - Encodage clé privée Age
- `x25519-dalek v2.0.1` - Courbe elliptique
- `rand v0.8.5` - Génération aléatoire
- `zeroize v1.8.1` - Effacement sécurisé mémoire

**Types JS/WASM** (à isoler) :
- `Uint8Array` (js_sys) - Transfert binaire JS ↔ Rust
- `AuthenticationExtensionsPrfValues` (web_sys) - Sortie WebAuthn PRF
- `JsValue` (wasm_bindgen) - Erreurs et valeurs JS
- `#[wasm_bindgen]` - Attribut pour exports WASM

### Problème

- ❌ Couplage fort aux types JS/WASM
- ❌ Impossible d'utiliser crypto.rs en dehors de WASM
- ❌ Pas de séparation ports/adapters (incohérent avec le reste du projet)
- ❌ Tous les retours sont `Result<T, JsValue>` au lieu d'erreurs typées

---

## 🏗️ Architecture Cible

```
src/
├── ports/
│   ├── crypto.rs                      // NEW: Traits crypto
│   └── mod.rs
│
├── adapters/
│   ├── shared/                        // NEW: Adapters identiques WASM/native
│   │   ├── age_encryption.rs          // Age fonctionne partout
│   │   ├── age_identity.rs            // Age fonctionne partout
│   │   ├── argon2_kdf.rs              // Argon2 fonctionne partout
│   │   └── mod.rs
│   │
│   ├── wasm/
│   │   ├── ... (existants)
│   │   └── webauthn_prf.rs            // NEW: PRF via WebAuthn (WASM only)
│   │
│   ├── native/
│   │   ├── ... (existants)
│   │   └── mock_prf.rs                // NEW: PRF stub (native)
│   │
│   └── mod.rs                         // Re-exports conditionnels
│
├── platform.rs                        // + méthodes crypto
│
├── domain/
│   └── crypto/
│       ├── operations.rs              // Logique métier
│       ├── types.rs                   // CryptoError
│       └── mod.rs
│
└── crypto.rs                          // WASM wrappers (IdentityHandle, RecipientHandle)
```

### Séparation des responsabilités

| Couche | Responsabilité | Types |
|--------|---------------|-------|
| **ports/crypto** | Interfaces (traits) | Traits seulement |
| **adapters/shared** | Implémentations communes | Age, Argon2 (Rust pur) |
| **adapters/wasm** | Implémentations WASM | WebAuthnPrf |
| **adapters/native** | Implémentations native | MockPrf |
| **domain/crypto** | Logique métier | CryptoError, opérations |
| **crypto.rs** | Bindings WASM | IdentityHandle, RecipientHandle, JsValue |

---

## 📋 Plan de Migration en 6 Étapes

### **ÉTAPE 1 : Ports crypto (traits)**

**Fichier** : `src/ports/crypto.rs`

```rust
use async_trait::async_trait;
use std::error::Error;

/// Port for encryption/decryption operations
#[async_trait(?Send)]
pub trait EncryptionPort: Send + Sync {
    /// Encrypt data for multiple recipients
    async fn encrypt(&self, data: &[u8], recipients: &[&str]) -> Result<Vec<u8>, Box<dyn Error>>;

    /// Decrypt data with an identity (private key string)
    async fn decrypt(&self, encrypted: &[u8], identity: &str) -> Result<Vec<u8>, Box<dyn Error>>;
}

/// Port for key derivation operations
#[async_trait(?Send)]
pub trait KeyDerivationPort: Send + Sync {
    /// Derive a 32-byte seed from a passphrase using Argon2
    async fn derive_from_passphrase(&self, passphrase: &str, salt: &[u8]) -> Result<[u8; 32], Box<dyn Error>>;
}

/// Port for identity management
pub trait IdentityPort: Send + Sync {
    /// Generate a new random identity
    fn generate(&self) -> Result<String, Box<dyn Error>>;

    /// Create identity from a 32-byte seed
    fn from_seed(&self, seed: [u8; 32]) -> Result<String, Box<dyn Error>>;

    /// Parse a recipient public key
    fn parse_recipient(&self, recipient: &str) -> Result<String, Box<dyn Error>>;

    /// Get public key from private identity
    fn to_public(&self, identity: &str) -> Result<String, Box<dyn Error>>;
}

/// Port for PRF (Pseudo-Random Function) operations
/// Only available in WASM (WebAuthn), stub in native
pub trait PrfPort: Send + Sync {
    /// Derive a 32-byte key from PRF outputs
    fn derive_from_prf(&self, first: &[u8], second: &[u8]) -> Result<[u8; 32], Box<dyn Error>>;

    /// Check if PRF is available on this platform
    fn is_available(&self) -> bool;
}
```

**Mise à jour** : `src/ports/mod.rs`

```rust
pub mod crypto;

pub use crypto::{EncryptionPort, IdentityPort, KeyDerivationPort, PrfPort};
```

**Tests** : Vérifier compilation des traits

**Commit** : `feat(crypto): add crypto ports (encryption, KDF, identity, PRF)`

---

### **ÉTAPE 2 : Shared adapters (Age + Argon2)**

Ces adapters sont **identiques** sur WASM et native car Age et Argon2 sont du Rust pur.

#### `src/adapters/shared/age_encryption.rs`

```rust
use crate::ports::EncryptionPort;
use age::{x25519::{Identity, Recipient}, Decryptor, Encryptor};
use async_trait::async_trait;
use futures::io::{AllowStdIo, AsyncReadExt, AsyncWriteExt};
use std::error::Error;
use std::io::Cursor;

/// Age encryption adapter - works on both WASM and native
#[derive(Clone, Copy, Debug)]
pub struct AgeEncryption;

impl AgeEncryption {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgeEncryption {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl EncryptionPort for AgeEncryption {
    async fn encrypt(&self, data: &[u8], recipients: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
        let parsed_recipients: Result<Vec<Recipient>, _> = recipients
            .iter()
            .map(|r| r.parse())
            .collect();
        let parsed = parsed_recipients?;

        if parsed.is_empty() {
            return Err("No recipients provided".into());
        }

        let encryptor = Encryptor::with_recipients(
            parsed.iter()
                .map(|r| Box::new(r.clone()) as Box<dyn age::Recipient + Send>)
                .collect()
        ).ok_or("Failed to create encryptor")?;

        let mut encrypted = vec![];
        let cursor = Cursor::new(&mut encrypted);
        let async_cursor = AllowStdIo::new(cursor);
        let mut writer = encryptor.wrap_output(Box::new(async_cursor))?;

        AsyncWriteExt::write_all(&mut writer, data).await?;
        AsyncWriteExt::close(&mut writer).await?;

        Ok(encrypted)
    }

    async fn decrypt(&self, encrypted: &[u8], identity_str: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let identity: Identity = identity_str.parse()?;

        let decryptor = Decryptor::new(encrypted)?;

        match decryptor {
            Decryptor::Recipients(d) => {
                let mut decrypted = vec![];
                let reader = d.decrypt(std::iter::once(&identity as &dyn age::Identity))?;
                let mut async_reader = AllowStdIo::new(reader);
                AsyncReadExt::read_to_end(&mut async_reader, &mut decrypted).await?;
                Ok(decrypted)
            }
            _ => Err("File was not encrypted with recipients".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt_decrypt_roundtrip() {
        let adapter = AgeEncryption::new();
        let identity = age::x25519::Identity::generate();
        let recipient = identity.to_public().to_string();

        let data = b"secret message";
        let encrypted = adapter.encrypt(data, &[&recipient]).await.unwrap();

        let identity_str = identity.to_string().expose_secret().to_string();
        let decrypted = adapter.decrypt(&encrypted, &identity_str).await.unwrap();

        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_encrypt_no_recipients() {
        let adapter = AgeEncryption::new();
        let data = b"secret";
        let result = adapter.encrypt(data, &[]).await;
        assert!(result.is_err());
    }
}
```

#### `src/adapters/shared/age_identity.rs`

```rust
use crate::ports::IdentityPort;
use age::x25519::{Identity, Recipient};
use age::secrecy::ExposeSecret;
use bech32::{ToBase32, Variant};
use x25519_dalek::StaticSecret;
use zeroize::Zeroize;
use std::error::Error;

/// Age identity adapter - works on both WASM and native
#[derive(Clone, Copy, Debug)]
pub struct AgeIdentity;

impl AgeIdentity {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgeIdentity {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentityPort for AgeIdentity {
    fn generate(&self) -> Result<String, Box<dyn Error>> {
        let identity = Identity::generate();
        Ok(identity.to_string().expose_secret().to_string())
    }

    fn from_seed(&self, seed: [u8; 32]) -> Result<String, Box<dyn Error>> {
        let mut seed_copy = seed;

        let secret = StaticSecret::from(seed_copy);
        let mut sk_bytes = secret.to_bytes();

        // Validate key bytes
        if sk_bytes.iter().all(|&x| x == 0) {
            sk_bytes.zeroize();
            seed_copy.zeroize();
            return Err("Generated invalid secret key (all zeros)".into());
        }

        let sk_base32 = sk_bytes.to_base32();
        let encoded = bech32::encode("age-secret-key-", sk_base32, Variant::Bech32)
            .map_err(|e| {
                sk_bytes.zeroize();
                seed_copy.zeroize();
                format!("Failed to encode identity: {}", e)
            })?
            .to_uppercase();

        sk_bytes.zeroize();
        seed_copy.zeroize();

        let identity: Identity = encoded.parse()
            .map_err(|e| format!("Failed to parse identity: {}", e))?;

        Ok(identity.to_string().expose_secret().to_string())
    }

    fn parse_recipient(&self, recipient_str: &str) -> Result<String, Box<dyn Error>> {
        let recipient: Recipient = recipient_str.parse()
            .map_err(|e| format!("Invalid recipient: {}", e))?;
        Ok(recipient.to_string())
    }

    fn to_public(&self, identity_str: &str) -> Result<String, Box<dyn Error>> {
        let identity: Identity = identity_str.parse()
            .map_err(|e| format!("Invalid identity: {}", e))?;
        Ok(identity.to_public().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let adapter = AgeIdentity::new();
        let identity_str = adapter.generate().unwrap();
        assert!(!identity_str.is_empty());
    }

    #[test]
    fn test_from_seed_deterministic() {
        let adapter = AgeIdentity::new();
        let seed = [42u8; 32];

        let identity1 = adapter.from_seed(seed).unwrap();
        let identity2 = adapter.from_seed(seed).unwrap();

        assert_eq!(identity1, identity2);
    }

    #[test]
    fn test_to_public() {
        let adapter = AgeIdentity::new();
        let identity = adapter.generate().unwrap();
        let public = adapter.to_public(&identity).unwrap();

        assert!(!public.is_empty());
        assert_ne!(identity, public);
    }
}
```

#### `src/adapters/shared/argon2_kdf.rs`

```rust
use crate::ports::KeyDerivationPort;
use argon2::Argon2;
use async_trait::async_trait;
use std::error::Error;

/// Argon2 key derivation adapter - works on both WASM and native
#[derive(Clone, Copy, Debug)]
pub struct Argon2Kdf;

impl Argon2Kdf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Argon2Kdf {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl KeyDerivationPort for Argon2Kdf {
    async fn derive_from_passphrase(&self, passphrase: &str, salt: &[u8]) -> Result<[u8; 32], Box<dyn Error>> {
        let argon2 = Argon2::default();
        let mut seed = [0u8; 32];
        argon2.hash_password_into(passphrase.as_bytes(), salt, &mut seed)?;
        Ok(seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_derive_is_deterministic() {
        let adapter = Argon2Kdf::new();
        let passphrase = "test password";
        let salt = b"test_salt_16byte";

        let seed1 = adapter.derive_from_passphrase(passphrase, salt).await.unwrap();
        let seed2 = adapter.derive_from_passphrase(passphrase, salt).await.unwrap();

        assert_eq!(seed1, seed2);
    }

    #[tokio::test]
    async fn test_different_passwords_different_seeds() {
        let adapter = Argon2Kdf::new();
        let salt = b"test_salt_16byte";

        let seed1 = adapter.derive_from_passphrase("password1", salt).await.unwrap();
        let seed2 = adapter.derive_from_passphrase("password2", salt).await.unwrap();

        assert_ne!(seed1, seed2);
    }
}
```

#### `src/adapters/shared/mod.rs`

```rust
//! Shared adapters that work identically on both WASM and native platforms.
//!
//! These adapters use pure Rust crates (Age, Argon2) that compile to both targets.

pub mod age_encryption;
pub mod age_identity;
pub mod argon2_kdf;

pub use age_encryption::AgeEncryption;
pub use age_identity::AgeIdentity;
pub use argon2_kdf::Argon2Kdf;
```

**Tests** : Tests unitaires pour chaque adapter shared

**Commit** : `feat(crypto): add shared crypto adapters (Age, Argon2)`

---

### **ÉTAPE 3 : PRF adapters (WASM vs native)**

Le PRF (WebAuthn) est spécifique à WASM. En native, on fournit un mock.

#### `src/adapters/wasm/webauthn_prf.rs`

```rust
use crate::ports::PrfPort;
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use std::error::Error;

/// WebAuthn PRF adapter - only available in WASM
#[derive(Clone, Copy, Debug)]
pub struct WebAuthnPrf;

impl WebAuthnPrf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebAuthnPrf {
    fn default() -> Self {
        Self::new()
    }
}

impl PrfPort for WebAuthnPrf {
    fn derive_from_prf(&self, first: &[u8], second: &[u8]) -> Result<[u8; 32], Box<dyn Error>> {
        if first.is_empty() {
            return Err("Missing first PRF value".into());
        }
        if second.is_empty() {
            return Err("Missing second PRF value".into());
        }

        let mut prf = first.to_vec();
        prf.extend(second);

        let mixed_prf = Sha256::digest(&prf);
        let (prk, _) = Hkdf::<Sha256>::extract(Some("hoddor/vault".as_bytes()), mixed_prf.as_slice());

        Ok(prk.into())
    }

    fn is_available(&self) -> bool {
        true
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_prf_derivation() {
        let adapter = WebAuthnPrf::new();
        let first = vec![1u8; 32];
        let second = vec![2u8; 32];

        let key = adapter.derive_from_prf(&first, &second).unwrap();
        assert_eq!(key.len(), 32);
    }

    #[wasm_bindgen_test]
    fn test_prf_is_available() {
        let adapter = WebAuthnPrf::new();
        assert!(adapter.is_available());
    }
}
```

#### `src/adapters/native/mock_prf.rs`

```rust
use crate::ports::PrfPort;
use std::error::Error;

/// Mock PRF adapter - PRF not available in native builds
#[derive(Clone, Copy, Debug)]
pub struct MockPrf;

impl MockPrf {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockPrf {
    fn default() -> Self {
        Self::new()
    }
}

impl PrfPort for MockPrf {
    fn derive_from_prf(&self, _first: &[u8], _second: &[u8]) -> Result<[u8; 32], Box<dyn Error>> {
        Err("PRF (WebAuthn) not available in native builds".into())
    }

    fn is_available(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prf_not_available() {
        let adapter = MockPrf::new();
        assert!(!adapter.is_available());
        assert!(adapter.derive_from_prf(&[1u8; 32], &[2u8; 32]).is_err());
    }
}
```

**Mise à jour** : `src/adapters/wasm/mod.rs`

```rust
// ... existing exports
pub mod webauthn_prf;
pub use webauthn_prf::WebAuthnPrf;
```

**Mise à jour** : `src/adapters/native/mod.rs`

```rust
// ... existing exports
pub mod mock_prf;
pub use mock_prf::MockPrf;
```

**Tests** : PRF tests sur WASM et native

**Commit** : `feat(crypto): add PRF adapters (WebAuthn for WASM, mock for native)`

---

### **ÉTAPE 4 : Intégration adapters**

#### `src/adapters/mod.rs`

```rust
/// Adapters module - platform-specific implementations of ports.

pub mod shared; // NEW: Shared adapters

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

// Re-export shared adapters (work on both platforms)
pub use shared::{AgeEncryption, AgeIdentity, Argon2Kdf};

// Platform-specific exports (existing)
#[cfg(target_arch = "wasm32")]
pub use wasm::{Clock, ConsoleLogger, Locks, Notifier, OPFSStorage as Storage, Persistence, WebAuthnPrf};

#[cfg(not(target_arch = "wasm32"))]
pub use native::{Clock, ConsoleLogger, FsStorage as Storage, Locks, Notifier, Persistence, MockPrf};
```

**Tests** : Vérifier imports conditionnels

**Commit** : `refactor(crypto): integrate crypto adapters into adapters module`

---

### **ÉTAPE 5 : Platform avec crypto**

#### `src/platform.rs`

```rust
/// Platform - Dependency injection container for all ports.

use crate::adapters::{
    AgeEncryption, AgeIdentity, Argon2Kdf, Clock, ConsoleLogger, Locks, Notifier, Persistence, Storage,
};

#[cfg(target_arch = "wasm32")]
use crate::adapters::WebAuthnPrf;
#[cfg(not(target_arch = "wasm32"))]
use crate::adapters::MockPrf;

use crate::ports::{
    ClockPort, EncryptionPort, IdentityPort, KeyDerivationPort, LockPort, LoggerPort, NotifierPort,
    PersistencePort, PrfPort, StoragePort,
};

#[derive(Clone, Copy)]
pub struct Platform {
    // Existing ports
    clock: Clock,
    logger: ConsoleLogger,
    locks: Locks,
    notifier: Notifier,
    persistence: Persistence,
    storage: Storage,

    // NEW: Crypto ports
    encryption: AgeEncryption,
    identity: AgeIdentity,
    kdf: Argon2Kdf,
    #[cfg(target_arch = "wasm32")]
    prf: WebAuthnPrf,
    #[cfg(not(target_arch = "wasm32"))]
    prf: MockPrf,
}

impl Platform {
    /// Creates a new Platform with default adapters for the current target.
    pub fn new() -> Self {
        Self {
            clock: Clock::new(),
            logger: ConsoleLogger::new(),
            locks: Locks::new(),
            notifier: Notifier::new(),
            persistence: Persistence::new(),
            storage: Storage::new(),
            encryption: AgeEncryption::new(),
            identity: AgeIdentity::new(),
            kdf: Argon2Kdf::new(),
            #[cfg(target_arch = "wasm32")]
            prf: WebAuthnPrf::new(),
            #[cfg(not(target_arch = "wasm32"))]
            prf: MockPrf::new(),
        }
    }

    // Existing port accessors
    #[inline]
    pub fn clock(&self) -> &dyn ClockPort {
        &self.clock
    }

    #[inline]
    pub fn logger(&self) -> &dyn LoggerPort {
        &self.logger
    }

    #[inline]
    pub fn locks(&self) -> &dyn LockPort {
        &self.locks
    }

    #[inline]
    pub fn persistence(&self) -> &dyn PersistencePort {
        &self.persistence
    }

    #[inline]
    pub fn storage(&self) -> &dyn StoragePort {
        &self.storage
    }

    #[inline]
    pub fn notifier(&self) -> &dyn NotifierPort {
        &self.notifier
    }

    // NEW: Crypto port accessors
    #[inline]
    pub fn encryption(&self) -> &dyn EncryptionPort {
        &self.encryption
    }

    #[inline]
    pub fn identity(&self) -> &dyn IdentityPort {
        &self.identity
    }

    #[inline]
    pub fn kdf(&self) -> &dyn KeyDerivationPort {
        &self.kdf
    }

    #[inline]
    pub fn prf(&self) -> &dyn PrfPort {
        &self.prf
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_crypto_access() {
        let platform = Platform::new();
        let _encryption = platform.encryption();
        let _identity = platform.identity();
        let _kdf = platform.kdf();
        let _prf = platform.prf();
    }
}
```

**Tests** : Accès crypto via Platform

**Commit** : `feat(crypto): integrate crypto ports into Platform`

---

### **ÉTAPE 6 : Domain crypto + refactor crypto.rs**

#### `src/domain/crypto/types.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum CryptoError {
    #[error("Key derivation failed: {0}")]
    KeyDerivationError(String),

    #[error("Encryption failed: {0}")]
    EncryptionError(String),

    #[error("Decryption failed: {0}")]
    DecryptionError(String),

    #[error("Invalid PRF output: {0}")]
    InvalidPrfOutput(String),

    #[error("Invalid identity: {0}")]
    InvalidIdentity(String),

    #[error("Invalid recipient: {0}")]
    InvalidRecipient(String),
}
```

#### `src/domain/crypto/operations.rs`

```rust
use crate::platform::Platform;
use super::types::CryptoError;

/// Derive an identity from a passphrase using Argon2 + Age
pub async fn identity_from_passphrase(
    platform: &Platform,
    passphrase: &str,
    salt: &[u8],
) -> Result<String, CryptoError> {
    let seed = platform
        .kdf()
        .derive_from_passphrase(passphrase, salt)
        .await
        .map_err(|e| CryptoError::KeyDerivationError(e.to_string()))?;

    platform
        .identity()
        .from_seed(seed)
        .map_err(|e| CryptoError::InvalidIdentity(e.to_string()))
}

/// Generate a new random identity
pub fn generate_identity(platform: &Platform) -> Result<String, CryptoError> {
    platform
        .identity()
        .generate()
        .map_err(|e| CryptoError::InvalidIdentity(e.to_string()))
}

/// Encrypt data for multiple recipients
pub async fn encrypt_for_recipients(
    platform: &Platform,
    data: &[u8],
    recipients: &[&str],
) -> Result<Vec<u8>, CryptoError> {
    platform
        .encryption()
        .encrypt(data, recipients)
        .await
        .map_err(|e| CryptoError::EncryptionError(e.to_string()))
}

/// Decrypt data with an identity
pub async fn decrypt_with_identity(
    platform: &Platform,
    encrypted_data: &[u8],
    identity: &str,
) -> Result<Vec<u8>, CryptoError> {
    platform
        .encryption()
        .decrypt(encrypted_data, identity)
        .await
        .map_err(|e| CryptoError::DecryptionError(e.to_string()))
}

/// Derive an identity from WebAuthn PRF outputs
pub async fn identity_from_prf(
    platform: &Platform,
    first: &[u8],
    second: &[u8],
) -> Result<String, CryptoError> {
    if !platform.prf().is_available() {
        return Err(CryptoError::InvalidPrfOutput(
            "PRF not available on this platform".to_string(),
        ));
    }

    let seed = platform
        .prf()
        .derive_from_prf(first, second)
        .map_err(|e| CryptoError::InvalidPrfOutput(e.to_string()))?;

    // Validate seed
    if seed.iter().all(|&x| x == 0) {
        return Err(CryptoError::InvalidPrfOutput("Invalid PRF seed (all zeros)".to_string()));
    }

    platform
        .identity()
        .from_seed(seed)
        .map_err(|e| CryptoError::InvalidIdentity(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_identity_from_passphrase() {
        let platform = Platform::new();
        let result = identity_from_passphrase(&platform, "test", b"salt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_roundtrip() {
        let platform = Platform::new();
        let identity = generate_identity(&platform).unwrap();
        let public = platform.identity().to_public(&identity).unwrap();

        let data = b"secret message";
        let encrypted = encrypt_for_recipients(&platform, data, &[&public]).await.unwrap();
        let decrypted = decrypt_with_identity(&platform, &encrypted, &identity).await.unwrap();

        assert_eq!(decrypted, data);
    }
}
```

#### `src/domain/crypto/mod.rs`

```rust
pub mod operations;
pub mod types;

pub use operations::{
    decrypt_with_identity, encrypt_for_recipients, generate_identity,
    identity_from_passphrase, identity_from_prf,
};
pub use types::CryptoError;
```

#### Refactor `src/crypto.rs`

```rust
// Conversion des wrappers WASM pour utiliser le domain
use crate::domain::crypto;
use crate::platform::Platform;
use age::x25519::{Identity, Recipient};
use wasm_bindgen::prelude::*;

// ... IdentityHandle et RecipientHandle restent inchangés ...

#[wasm_bindgen]
pub async fn identity_from_passphrase(
    passphrase: &str,
    salt: &[u8],
) -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();

    let identity_str = crypto::identity_from_passphrase(&platform, passphrase, salt)
        .await
        .map_err(|e| {
            platform.logger().log(&format!("Failed to derive identity: {}", e));
            JsValue::from_str(&e.to_string())
        })?;

    let identity: Identity = identity_str.parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    Ok(IdentityHandle::from(identity))
}

#[wasm_bindgen]
pub fn generate_identity() -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();

    let identity_str = crypto::generate_identity(&platform)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let identity: Identity = identity_str.parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    Ok(IdentityHandle::from(identity))
}

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

// Conversion PRF: AuthenticationExtensionsPrfValues → Vec<u8>
fn prf_outputs_from_js(
    prf: &web_sys::AuthenticationExtensionsPrfValues
) -> Result<(Vec<u8>, Vec<u8>), JsValue> {
    let first = if !prf.get_first().is_undefined() {
        js_sys::Uint8Array::new(&prf.get_first()).to_vec()
    } else {
        return Err(JsValue::from_str("Missing first PRF value"));
    };

    let second = if let Some(s) = prf.get_second() {
        js_sys::Uint8Array::new(&s).to_vec()
    } else {
        return Err(JsValue::from_str("Missing second PRF value"));
    };

    Ok((first, second))
}

pub fn identity_from_prf(
    prf_output: &web_sys::AuthenticationExtensionsPrfValues,
) -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();
    let (first, second) = prf_outputs_from_js(prf_output)?;

    let identity_str = crypto::identity_from_prf(&platform, &first, &second)
        .map_err(|e| {
            platform.logger().log(&format!("Failed to derive identity from PRF: {}", e));
            JsValue::from_str(&e.to_string())
        })?;

    let identity: Identity = identity_str.parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    let handle = IdentityHandle::from(identity);

    // Validate
    if handle.public_key().is_empty() || handle.private_key().is_empty() {
        return Err(JsValue::from_str("Generated invalid identity handle"));
    }

    Ok(handle)
}
```

**Commit 1** : `feat(crypto): add domain crypto operations`
**Commit 2** : `refactor(crypto): convert to WASM wrappers using domain`

---

## ✅ Résultats Attendus

### Métriques

| Métrique | Avant | Après |
|----------|-------|-------|
| **Lignes crypto.rs** | 350 | ~150 (wrappers WASM) |
| **Lignes domain/crypto** | 0 | ~200 (logique métier) |
| **Lignes adapters shared** | 0 | ~300 (Age, Argon2) |
| **Lignes adapters wasm** | 0 | ~50 (WebAuthnPrf) |
| **Lignes adapters native** | 0 | ~30 (MockPrf) |
| **Code dupliqué** | N/A | 0 |
| **Testable hors WASM** | ❌ Non | ✅ Oui |
| **Interopérable** | ❌ WASM only | ✅ WASM + native |

### Avantages

1. ✅ **Cohérence architecturale** : Suit le pattern ports/adapters existant
2. ✅ **Interopérabilité** : Utilisable en CLI, serveur, lib Rust native
3. ✅ **0 duplication** : Adapters partagés entre WASM et native
4. ✅ **Testabilité** : Tests unitaires sans WASM
5. ✅ **Flexibilité** : Changement d'implémentation crypto sans toucher le domain
6. ✅ **Séparation claire** : WebAuthn PRF isolé dans WASM uniquement
7. ✅ **Erreurs typées** : `CryptoError` au lieu de `JsValue`

### Validation Finale

**Critères de succès** :
- ✅ `domain/crypto` compile sans `target = "wasm32"`
- ✅ Tous les tests domaine passent en mode natif
- ✅ crypto.rs WASM fonctionne (tests d'intégration)
- ✅ Pas de duplication de logique crypto
- ✅ Erreurs typées et informatives
- ✅ Zeroization préservée pour données sensibles

**Test d'interopérabilité** :
```rust
// Dans tests/crypto_native.rs (test hors WASM)
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_encrypt_decrypt_native() {
    let platform = Platform::new();
    let identity = crypto::generate_identity(&platform).unwrap();
    let public = platform.identity().to_public(&identity).unwrap();
    let data = b"secret message";

    let encrypted = crypto::encrypt_for_recipients(&platform, data, &[&public])
        .await
        .unwrap();
    let decrypted = crypto::decrypt_with_identity(&platform, &encrypted, &identity)
        .await
        .unwrap();

    assert_eq!(decrypted, data);
}
```

---

## 📌 Ordre d'Exécution

1. **Étape 1** → Ports (traits)
2. **Étape 2** → Shared adapters (Age, Argon2)
3. **Étape 3** → PRF adapters (WASM/native)
4. **Étape 4** → Intégration adapters
5. **Étape 5** → Platform avec crypto
6. **Étape 6** → Domain crypto + refactor crypto.rs

**Durée estimée** : 3-4h avec tests complets

---

## 🔍 Comparaison shared/ vs duplication

| Aspect | Sans shared | Avec shared |
|--------|-------------|-------------|
| **Code dupliqué** | ~300 lignes × 2 | 0 ligne dupliquée |
| **Maintenance** | Changer 2 fichiers | Changer 1 fichier |
| **Tests** | Tests × 2 plateformes | Tests une fois |
| **Cohérence** | Risque de divergence | Garanti identique |
| **Clarté** | Où est la logique ? | `shared/` = multi-plateforme |
