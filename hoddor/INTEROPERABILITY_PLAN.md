# Plan d'interopérabilité WASM/Native

## 🎯 Objectif

Rendre la librairie Hoddor complètement interopérable entre Rust pur (native) et WASM, en séparant les entry points (façades) de la logique métier (domain).

**Principe** : Différentes façades (WASM, Native) qui consomment les mêmes fonctions domain derrière.

## 📊 Architecture actuelle

### Structure existante

```
src/
├── domain/          ✅ Logique métier pure
│   ├── crypto/
│   └── vault/
├── ports/           ✅ Interfaces abstraites
│   ├── logger.rs
│   ├── storage.rs
│   ├── crypto.rs
│   ├── lock.rs
│   ├── persistence.rs
│   ├── notifier.rs
│   └── clock.rs
├── adapters/        ✅ Implémentations WASM/Native
│   ├── wasm/
│   └── native/
├── vault.rs         ❌ Mélange façade WASM + logique
├── crypto.rs        ✅ Façade WASM propre
└── webauthn/        ⚠️  Façade WASM + logique
```

### Le problème : vault.rs (1025 lignes)

**11 fonctions WASM exportées** (hors sync/webrtc) :

| Fonction | Responsabilité | Couplage WASM |
|----------|---------------|---------------|
| `vault_identity_from_passphrase` | Dérivation identité | `JsValue`, `js_sys::Date` |
| `upsert_vault` | Insertion/MAJ données | `JsValue`, `serde_wasm_bindgen` |
| `remove_from_vault` | Suppression namespace | `JsValue`, `serde_wasm_bindgen` |
| `read_from_vault` | Lecture données | `JsValue`, `serde_wasm_bindgen` |
| `list_namespaces` | Liste namespaces | `JsValue`, `serde_wasm_bindgen` |
| `remove_vault` | Suppression vault | `JsValue` |
| `list_vaults` | Liste vaults | `JsValue`, `serde_wasm_bindgen` |
| `create_vault` | Création vault | `JsValue` |
| `export_vault` | Export binaire | `JsValue`, `js_sys::Uint8Array` |
| `import_vault` | Import binaire | `JsValue`, `js_sys::Uint8Array` |
| `force_cleanup_vault` | Nettoyage expirations | `JsValue`, `js_sys::Date` |

**Pattern répétitif détecté** :
```rust
// Fonction publique WASM
#[wasm_bindgen]
pub async fn function_name(...) -> Result<JsValue, JsValue> {
    let platform = Platform::new();
    function_name_internal(&platform, ...).await
}

// Fonction interne avec Platform
async fn function_name_internal(
    platform: &Platform,
    ...
) -> Result<..., VaultError> {
    // Logique métier + conversions WASM
    // ❌ Impossible d'utiliser en Rust natif
}
```

### État du domaine actuel

**domain/vault/** contient déjà de la logique pure :
- ✅ `operations.rs` : read_vault, save_vault, list_vaults, delete_vault, create_vault
- ✅ `validation.rs` : validate_vault_name, validate_namespace, validate_passphrase
- ✅ `serialization.rs` : serialize_vault, deserialize_vault
- ✅ `expiration.rs` : is_expired, cleanup_expired_namespaces
- ✅ `types.rs` : Vault, VaultMetadata, NamespaceData, IdentitySalts
- ✅ `error.rs` : VaultError

**Manque** : Fonctions domain pour upsert, remove, read namespaces avec identités.

## 🎯 Plan d'action

### Phase 1 : Créer les nouveaux domaines et ports

#### 1.1 Nouveau domaine : `authentication`

**Objectif** : Isoler la logique de dérivation et gestion d'identités.

```
src/domain/authentication/
├── mod.rs
├── operations.rs  # Fonctions pures de dérivation
├── error.rs       # AuthenticationError
└── types.rs       # Types agnostiques
```

**Fonctions à créer** :
```rust
// operations.rs
pub async fn derive_vault_identity(
    platform: &Platform,
    passphrase: &str,
    vault_name: &str,
) -> Result<IdentityKeys, AuthenticationError>

pub async fn find_or_create_identity(
    platform: &Platform,
    passphrase: &str,
    vault: &mut Vault,
) -> Result<IdentityKeys, AuthenticationError>
```

**Types agnostiques** :
```rust
// types.rs
#[derive(Clone)]
pub struct IdentityKeys {
    pub public_key: String,
    pub private_key: String,
}
```

#### 1.2 Nouveau port : `identity_provider`

**Objectif** : Interface abstraite pour génération/dérivation d'identités.

```rust
// src/ports/identity_provider.rs
use async_trait::async_trait;
use super::error::CryptoError;

#[async_trait(?Send)]
pub trait IdentityProvider {
    /// Dérive une identité depuis une passphrase et un salt
    async fn derive_identity(
        &self,
        passphrase: &str,
        salt: &[u8],
    ) -> Result<String, CryptoError>;

    /// Génère une nouvelle identité aléatoire
    fn generate_identity(&self) -> Result<String, CryptoError>;
}
```

**Implémentations** :
- `adapters/wasm/identity_provider.rs` : utilise argon2 + age
- `adapters/native/identity_provider.rs` : même logique sans WASM

### Phase 2 : Enrichir domain/vault/operations.rs

**Objectif** : Ajouter des fonctions pures pour manipuler les namespaces.

#### 2.1 Nouvelles fonctions domain

```rust
// src/domain/vault/operations.rs

/// Insère ou met à jour un namespace dans un vault
pub async fn upsert_namespace(
    platform: &Platform,
    vault_name: &str,
    identity_public_key: &str,
    namespace: &str,
    data: Vec<u8>,
    expires_in_seconds: Option<i64>,
    replace_if_exists: bool,
) -> Result<(), VaultError>

/// Lit les données d'un namespace
pub async fn read_namespace(
    platform: &Platform,
    vault_name: &str,
    identity_private_key: &str,
    namespace: &str,
) -> Result<Vec<u8>, VaultError>

/// Supprime un namespace d'un vault
pub async fn remove_namespace(
    platform: &Platform,
    vault_name: &str,
    namespace: &str,
) -> Result<(), VaultError>

/// Liste les namespaces d'un vault
pub async fn list_namespaces_in_vault(
    platform: &Platform,
    vault_name: &str,
) -> Result<Vec<String>, VaultError>

/// Exporte un vault en bytes
pub async fn export_vault_bytes(
    platform: &Platform,
    vault_name: &str,
) -> Result<Vec<u8>, VaultError>

/// Importe un vault depuis bytes
pub async fn import_vault_from_bytes(
    platform: &Platform,
    vault_name: &str,
    vault_bytes: &[u8],
) -> Result<(), VaultError>

/// Nettoie les namespaces expirés d'un vault
pub async fn cleanup_vault(
    platform: &Platform,
    vault_name: &str,
) -> Result<bool, VaultError>

/// Vérifie qu'une identité peut déchiffrer un vault
pub async fn verify_vault_identity(
    platform: &Platform,
    vault_name: &str,
    identity_private_key: &str,
) -> Result<(), VaultError>
```

**Principes des fonctions domain** :
- ✅ Prennent `Platform` en paramètre (injection de dépendances)
- ✅ Utilisent `Vec<u8>` au lieu de `JsValue`
- ✅ Utilisent `String` pour les clés publiques/privées
- ✅ Retournent `Result<T, VaultError>` (pas de `JsValue`)
- ✅ Zéro dépendance WASM
- ✅ Utilisables en Rust natif

#### 2.2 Helpers de conversion

```rust
// src/domain/vault/data.rs

/// Chiffre des données pour un recipient
pub async fn encrypt_data(
    platform: &Platform,
    data: &[u8],
    recipient_public_key: &str,
    expires_in_seconds: Option<i64>,
) -> Result<NamespaceData, VaultError>

/// Déchiffre les données d'un namespace
pub async fn decrypt_namespace_data(
    platform: &Platform,
    namespace_data: &NamespaceData,
    identity_private_key: &str,
) -> Result<Vec<u8>, VaultError>
```

### Phase 3 : Créer les façades

#### 3.1 Structure des façades

```
src/facades/
├── mod.rs
├── wasm/
│   ├── mod.rs
│   ├── vault.rs        # Façade WASM pour vault
│   ├── crypto.rs       # Déplacer depuis src/crypto.rs
│   ├── webauthn.rs     # Déplacer depuis src/webauthn/
│   └── converters.rs   # Utilitaires JsValue ↔ Rust
└── native/
    ├── mod.rs
    ├── vault.rs        # API Rust native pour vault
    ├── crypto.rs       # API native pour crypto
    └── webauthn.rs     # API native pour webauthn
```

#### 3.2 Exemple : Façade WASM

```rust
// src/facades/wasm/vault.rs
use wasm_bindgen::prelude::*;
use crate::crypto::IdentityHandle;
use crate::platform::Platform;
use crate::domain::vault::operations;
use super::converters;

#[wasm_bindgen]
pub async fn upsert_vault(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: &str,
    data: JsValue,
    expires_in_seconds: Option<i64>,
    replace_if_exists: bool,
) -> Result<(), JsValue> {
    let platform = Platform::new();

    // Validation
    crate::domain::vault::validation::validate_namespace(namespace)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Conversion WASM → Rust
    let data_bytes = converters::js_value_to_bytes(data)?;

    // Appel du domain (logique pure)
    operations::upsert_namespace(
        &platform,
        vault_name,
        &identity.public_key(),
        namespace,
        data_bytes,
        expires_in_seconds,
        replace_if_exists,
    )
    .await
    .map_err(|e| e.into())
}

#[wasm_bindgen]
pub async fn read_from_vault(
    vault_name: &str,
    identity: &IdentityHandle,
    namespace: JsValue,
) -> Result<JsValue, JsValue> {
    let platform = Platform::new();

    // Conversion WASM → Rust
    let namespace_str = converters::js_value_to_string(namespace)?;

    // Appel du domain
    let data_bytes = operations::read_namespace(
        &platform,
        vault_name,
        &identity.private_key(),
        &namespace_str,
    )
    .await
    .map_err(|e| e.into())?;

    // Conversion Rust → WASM
    converters::bytes_to_js_value(&data_bytes)
}

#[wasm_bindgen]
pub async fn vault_identity_from_passphrase(
    passphrase: &str,
    vault_name: &str,
) -> Result<IdentityHandle, JsValue> {
    let platform = Platform::new();

    // Appel du domain authentication
    let identity_keys = crate::domain::authentication::operations::derive_vault_identity(
        &platform,
        passphrase,
        vault_name,
    )
    .await
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Conversion vers IdentityHandle (type WASM)
    converters::identity_keys_to_handle(identity_keys)
}
```

#### 3.3 Exemple : Façade Native

```rust
// src/facades/native/vault.rs
use crate::platform::Platform;
use crate::domain::vault::{VaultError, operations};

pub struct VaultManager {
    platform: Platform,
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            platform: Platform::new(),
        }
    }

    pub async fn upsert_namespace(
        &self,
        vault_name: &str,
        identity_public_key: &str,
        namespace: &str,
        data: Vec<u8>,
        expires_in_seconds: Option<i64>,
        replace_if_exists: bool,
    ) -> Result<(), VaultError> {
        operations::upsert_namespace(
            &self.platform,
            vault_name,
            identity_public_key,
            namespace,
            data,
            expires_in_seconds,
            replace_if_exists,
        ).await
    }

    pub async fn read_namespace(
        &self,
        vault_name: &str,
        identity_private_key: &str,
        namespace: &str,
    ) -> Result<Vec<u8>, VaultError> {
        operations::read_namespace(
            &self.platform,
            vault_name,
            identity_private_key,
            namespace,
        ).await
    }

    pub async fn list_namespaces(
        &self,
        vault_name: &str,
    ) -> Result<Vec<String>, VaultError> {
        operations::list_namespaces_in_vault(
            &self.platform,
            vault_name,
        ).await
    }

    // ... autres méthodes
}
```

#### 3.4 Module de conversion WASM

```rust
// src/facades/wasm/converters.rs
use wasm_bindgen::prelude::*;
use js_sys::Uint8Array;
use serde_wasm_bindgen::{from_value, to_value};

/// Convertit JsValue → Vec<u8>
pub fn js_value_to_bytes(value: JsValue) -> Result<Vec<u8>, JsValue> {
    if value.is_instance_of::<Uint8Array>() {
        let array = Uint8Array::from(value);
        Ok(array.to_vec())
    } else {
        // Tenter de désérialiser comme JSON
        let json: serde_json::Value = from_value(value)?;
        serde_json::to_vec(&json)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize: {}", e)))
    }
}

/// Convertit Vec<u8> → JsValue
pub fn bytes_to_js_value(bytes: &[u8]) -> Result<JsValue, JsValue> {
    // Tenter de parser comme JSON
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(json_value) => to_value(&json_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert: {}", e))),
        Err(_) => {
            // Retourner comme Uint8Array
            let array = Uint8Array::new_with_length(bytes.len() as u32);
            array.copy_from(bytes);
            Ok(array.into())
        }
    }
}

/// Convertit JsValue → String
pub fn js_value_to_string(value: JsValue) -> Result<String, JsValue> {
    value.as_string()
        .or_else(|| from_value::<String>(value.clone()).ok())
        .ok_or_else(|| JsValue::from_str("Invalid string value"))
}

/// Convertit IdentityKeys → IdentityHandle
pub fn identity_keys_to_handle(
    keys: crate::domain::authentication::types::IdentityKeys
) -> Result<crate::crypto::IdentityHandle, JsValue> {
    use age::x25519::Identity;

    let identity: Identity = keys.private_key.parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse identity: {}", e)))?;

    Ok(crate::crypto::IdentityHandle::from(identity))
}
```

### Phase 4 : Migration progressive

#### Ordre d'exécution

```
1. ✅ Créer domain/authentication/
   ├── operations.rs
   ├── types.rs
   └── error.rs

2. ✅ Créer ports/identity_provider.rs
   ├── Trait IdentityProvider
   └── Implémentations dans adapters/

3. ✅ Enrichir domain/vault/operations.rs
   ├── upsert_namespace()
   ├── read_namespace()
   ├── remove_namespace()
   ├── list_namespaces_in_vault()
   ├── export_vault_bytes()
   ├── import_vault_from_bytes()
   ├── cleanup_vault()
   └── verify_vault_identity()

4. ✅ Créer domain/vault/data.rs
   ├── encrypt_data()
   └── decrypt_namespace_data()

5. ✅ Créer facades/wasm/
   ├── converters.rs (utilitaires)
   ├── vault.rs (11 fonctions WASM)
   └── mod.rs

6. ✅ Migrer une fonction test
   └── upsert_vault() comme POC

7. ✅ Migrer les 10 autres fonctions vault

8. ✅ Déplacer src/crypto.rs → facades/wasm/crypto.rs

9. ✅ Déplacer src/webauthn/ → facades/wasm/webauthn/

10. ✅ Créer facades/native/
    ├── vault.rs (VaultManager)
    ├── crypto.rs (CryptoManager)
    └── webauthn.rs (WebAuthnManager)

11. ✅ Supprimer l'ancien src/vault.rs

12. ✅ Tests d'intégration
    ├── WASM : vérifier que playground fonctionne
    └── Native : créer tests Rust purs
```

#### Migration incrémentale

**Étape 1** : Fonction POC (upsert_vault)
- Créer la fonction domain `upsert_namespace()`
- Créer la façade WASM `upsert_vault()`
- Tester que ça compile en WASM
- Vérifier dans playground

**Étape 2** : Migration par groupe
- **Groupe 1 - CRUD** : upsert, read, remove, list_namespaces
- **Groupe 2 - Gestion vault** : create, remove_vault, list_vaults
- **Groupe 3 - Import/Export** : export_vault, import_vault
- **Groupe 4 - Maintenance** : force_cleanup_vault
- **Groupe 5 - Authentication** : vault_identity_from_passphrase

**Étape 3** : Validation
- ✅ Tous les tests WASM passent
- ✅ Le playground fonctionne
- ✅ Les types générés (.d.ts) sont identiques
- ✅ Supprimer src/vault.rs

## 📁 Structure cible finale

```
src/
├── domain/                    # Logique métier pure (0 dépendance WASM)
│   ├── authentication/
│   │   ├── mod.rs
│   │   ├── operations.rs
│   │   ├── types.rs
│   │   └── error.rs
│   ├── crypto/
│   │   ├── mod.rs
│   │   ├── operations.rs
│   │   └── error.rs
│   └── vault/
│       ├── mod.rs
│       ├── types.rs
│       ├── error.rs
│       ├── validation.rs
│       ├── expiration.rs
│       ├── serialization.rs
│       ├── operations.rs      # Enrichi avec 8 nouvelles fonctions
│       └── data.rs            # Nouveau : encrypt/decrypt helpers
│
├── ports/                     # Interfaces abstraites
│   ├── mod.rs
│   ├── logger.rs
│   ├── storage.rs
│   ├── crypto.rs
│   ├── lock.rs
│   ├── persistence.rs
│   ├── notifier.rs
│   ├── clock.rs
│   └── identity_provider.rs   # Nouveau port
│
├── adapters/                  # Implémentations
│   ├── mod.rs
│   ├── wasm/
│   │   ├── ...
│   │   └── identity_provider.rs
│   └── native/
│       ├── ...
│       └── identity_provider.rs
│
├── facades/                   # Entry points (séparation WASM/Native)
│   ├── mod.rs
│   ├── wasm/                  # API JavaScript/WASM
│   │   ├── mod.rs
│   │   ├── vault.rs           # 11 fonctions #[wasm_bindgen]
│   │   ├── crypto.rs          # Déplacé depuis src/crypto.rs
│   │   ├── webauthn/          # Déplacé depuis src/webauthn/
│   │   │   ├── mod.rs
│   │   │   ├── webauthn.rs
│   │   │   └── crypto_helpers.rs
│   │   └── converters.rs      # JsValue ↔ Rust
│   └── native/                # API Rust native
│       ├── mod.rs
│       ├── vault.rs           # VaultManager
│       ├── crypto.rs          # CryptoManager
│       └── webauthn.rs        # WebAuthnManager (si applicable)
│
├── platform.rs
├── global.rs
├── measure.rs
└── lib.rs
```

## ✅ Avantages de cette architecture

### Séparation des préoccupations
- **Domain** : Logique métier testable en Rust pur
- **Ports** : Contrats d'interface
- **Adapters** : Implémentations spécifiques (WASM/Native)
- **Facades** : Entry points minimalistes (conversion + délégation)

### Interopérabilité complète
- ✅ Même logique pour WASM et Native
- ✅ API Rust native utilisable dans des binaires
- ✅ API WASM pour JavaScript/TypeScript
- ✅ Tests unitaires sans dépendances WASM

### Maintenabilité
- ✅ Logique métier centralisée dans domain
- ✅ Façades légères (30-50 lignes par fonction)
- ✅ Ajout facile de nouvelles façades (CLI, FFI, etc.)
- ✅ Refactoring domain sans casser les façades

### Testabilité
- ✅ Tests domain en Rust pur (rapides, pas de WASM)
- ✅ Tests façades WASM avec wasm-bindgen-test
- ✅ Tests intégration Native
- ✅ Mocks des ports facilités

## 🚫 Hors scope (à ne pas toucher)

Les fonctions suivantes **ne seront pas migrées** pour le moment :
- `enable_sync()` - Activation synchronisation
- `connect_to_peer()` - Connexion WebRTC
- `add_peer()` - Gestion permissions pair
- `update_vault_from_sync()` - Réception sync

**Raison** : Ces fonctions dépendent de WebRTC/signaling qui nécessitent un refactoring complet séparé.

## 📝 Prochaines étapes

1. Valider ce plan d'action
2. Commencer par la Phase 1 : créer domain/authentication
3. Implémenter la fonction POC (upsert_vault)
4. Migration incrémentale des 10 autres fonctions
5. Documentation de la nouvelle API native
