# Hexagonal Architecture

## Architecture Type
Hexagonal (Ports & Adapters) - **WASM-focused** with native adapters for testing only

## Directory Structure

```
src/
├── domain/              # Business logic (pure, no infrastructure deps)
│   ├── vault/          # Vault operations, types, validation
│   ├── crypto/         # Encryption operations
│   └── authentication/ # Identity derivation
├── ports/              # Abstract interfaces (traits)
│   ├── storage.rs      # StoragePort, LockPort, PersistencePort
│   ├── crypto.rs       # EncryptionPort, KeyDerivationPort, PrfPort
│   ├── clock.rs        # ClockPort
│   ├── logger.rs       # LoggerPort
│   └── notifier.rs     # NotifierPort
├── adapters/           # Concrete implementations
│   ├── wasm/          # Production: OPFS, WebAuthn, Web Locks
│   ├── native/        # Testing only: stubs and mocks
│   └── shared/        # Both platforms: Age, Argon2
├── platform.rs         # Dependency injection container
├── facades/
│   └── wasm/          # WASM bindings for JavaScript (wasm_bindgen)
└── [legacy modules]   # Global, sync, webrtc (WASM-only)
```

## Key Architectural Decisions

### 1. WASM is Primary Target
- **Production adapters**: WASM only (OPFS storage, WebAuthn PRF, Web Locks API)
- **Native adapters**: Testing infrastructure only (simple stubs/mocks)
- **Reason**: Browser vault - native is for unit tests, not production use

### 2. Platform Injection Pattern
- Domain functions accept `&Platform` parameter (not individual ports)
- Platform provides access to all ports via trait methods
- **Trade-off**: Pragmatic approach vs pure hexagonal (simpler but couples domain to Platform)

### 3. Shared Cryptography
- Age encryption and Argon2 KDF work on both platforms
- Ensures data compatibility between WASM tests and native unit tests

## Core Design Patterns

### Port Definition Rules
**Location**: `src/ports/[name].rs`

**Requirements**:
- `Send + Sync` bounds (async and multi-threading compatibility)
- `&self` methods (stateless or internal mutability)
- Borrowed types preferred (`&str`, `&[u8]`)
- Single responsibility per port
- Use `async_trait(?Send)` for async methods (WASM compatibility)

**Current Issue**: Some ports return `VaultError` (domain type) creating reverse dependency. Should use generic error types.

### Adapter Implementation Rules
**Locations**: `src/adapters/wasm/` (production), `src/adapters/native/` (testing)

**Requirements**:
- `#[derive(Clone, Copy)]` on all adapters (zero-sized types)
- Simple `new()` constructor
- Implements corresponding port trait
- WASM adapters use `web_sys`, `js_sys` APIs
- Native adapters are simple stubs/mocks

**Platform Selection**:
- `#[cfg(target_arch = "wasm32")]` for WASM modules
- `#[cfg(not(target_arch = "wasm32"))]` for native modules
- Compile-time selection, zero-cost abstraction

### Platform Container Pattern
**Location**: `src/platform.rs`

**Purpose**: Dependency injection container holding all adapter instances

**Structure**:
- Stores concrete adapter instances (not trait objects)
- `#[derive(Clone, Copy)]` (all adapters are zero-sized)
- Accessor methods return `&dyn Trait` references
- Created via `Platform::new()` - instantiates all adapters

**Usage**:
- Store as field in structs with state
- Create once at entry points (WASM facades)
- Pass by reference to domain functions
- Copy for closures (Platform is Copy)

**Trade-off**: Domain depends on Platform instead of individual ports (pragmatic but less pure hexagonal)

## Current Ports and Adapters

### Storage Layer
- **StoragePort**: File system operations (read, write, delete, list, directories)
  - WASM: OPFS (Origin Private File System) via web_sys
  - Native: Simple in-memory HashMap mock for testing

### Cryptography
- **EncryptionPort**: Age encryption/decryption with multi-recipient support
- **KeyDerivationPort**: Argon2 key derivation from passphrases
- **IdentityPort**: Age identity generation and management
- **PrfPort**: WebAuthn PRF (Pseudo-Random Function) key derivation
  - WASM: WebAuthn API with HKDF
  - Native: Stub (returns mock data, `is_available()` returns false)

**Shared**: Age and Argon2 implementations work on both platforms

### Synchronization
- **LockPort**: Exclusive lock acquisition with retry logic
  - WASM: Web Locks API with exponential backoff
  - Native: No-op stub (returns immediately - **unsafe for concurrent native use**)

### Infrastructure
- **ClockPort**: Timestamp access
  - WASM: `js_sys::Date::now()`
  - Native: `SystemTime::now()`
- **LoggerPort**: Logging operations (log, warn, error, time)
  - WASM: `console.log` via web_sys
  - Native: `println!` / `eprintln!`
- **PersistencePort**: Storage persistence requests (browser API)
  - WASM: StorageManager persist() request
  - Native: Always returns true (stub)
- **NotifierPort**: Cross-context notifications
  - WASM: postMessage API
  - Native: No-op (single process)

## WASM Facades Layer

**Purpose**: JavaScript interop via `wasm_bindgen`

**Location**: `src/facades/wasm/`

**Pattern**:
- Functions decorated with `#[wasm_bindgen]`
- Accept/return `JsValue` types
- Create Platform instance at entry point
- Call domain operations
- Convert results back to JsValue

**Modules**:
- **vault.rs**: All vault operations (create, read, upsert, delete, import, export)
- **crypto.rs**: Identity generation, encryption operations
- **webauthn/**: WebAuthn PRF operations and credential management
- **converters.rs**: Helper functions for Rust ↔ JS conversions

**Error Handling**: All errors converted to `JsValue` at facade boundary

## Testing Strategy

### Test Hierarchy (No Redundancy)

**Three Levels**:
1. **Platform** (integration): 1 test per port - verify accessibility only
2. **WASM Adapters** (unit): 3-5 tests - WASM-specific implementation
3. **Native Adapters** (unit): 3-5 tests - native-specific implementation

**Domain Tests**: Test business logic independently with Platform

**Integration Tests**: Full end-to-end scenarios (`tests/vault_operations.rs`)
- WASM-only (`#[cfg(target_arch = "wasm32")]`)
- Comprehensive coverage: CRUD, concurrency, expiration, edge cases
- 94 unit tests + extensive integration suite

### Test Execution
- `cargo test --lib` - Native unit tests
- `wasm-pack test --headless --chrome` - WASM tests
- Tests use `Platform::new()` (no mocking needed)

### Key Testing Decisions

**No Mocking**: Tests use real adapters
- Native tests use simple stub implementations
- WASM tests run in actual browser environment
- Integration tests validate full system behavior

**Test Isolation**: Each test cleans up (`test_utils::cleanup_all_vaults()`)

**Known Issues**:
- One ignored test due to Age library i18n bug in WASM (invalid password test)

## Domain Layer Details

### Vault Module (`domain/vault/`)

**Core Responsibilities**:
- Vault lifecycle (create, read, save, delete)
- Namespace operations (upsert, read, remove, list)
- Data expiration management
- Import/export with custom serialization format
- Input validation (vault names, namespaces, passphrases)

**Key Files**:
- **types.rs**: Core data structures
  - `Vault`: Main container (metadata, salts, namespaces, username mappings)
  - `NamespaceData`: Encrypted data + optional expiration
  - `VaultMetadata`: Peer ID for sync
  - `IdentitySalts`: Salt management for key derivation
- **operations.rs**: All vault CRUD operations
- **validation.rs**: Input validation rules
- **expiration.rs**: Automatic cleanup of expired data
- **serialization.rs**: Custom binary format with magic number (`0x484F444F` = "HODO")
- **error.rs**: Domain-specific error types

**Storage Model**:
```
vault_name/
├── metadata.json       # Vault metadata (no namespaces)
├── namespace1.ns       # Individual namespace files
├── namespace2.ns
└── ...
```

### Crypto Module (`domain/crypto/`)

**Responsibilities**:
- Age encryption/decryption for multiple recipients
- Identity generation (random or from seed)
- Recipient public key parsing

**Integration**: Uses ports (EncryptionPort, IdentityPort, KeyDerivationPort)

### Authentication Module (`domain/authentication/`)

**Responsibilities**:
- Identity derivation from passphrase
- Salt management per vault
- Key derivation flow: Passphrase → Argon2 → Seed → Age Identity

**Pattern**: Each vault has unique salt stored in `IdentitySalts`, deterministic key derivation

## Data Flow Examples

### Creating and Writing to Vault

```
JavaScript
    ↓ (wasm_bindgen call)
facades/wasm/vault.rs::create_vault()
    ↓ Platform::new()
    ↓ Convert JsValue → Rust types
domain/vault/operations.rs::create_vault()
    ↓ Create Vault struct
domain/vault/operations.rs::save_vault()
    ↓ platform.storage() (StoragePort)
adapters/wasm/opfs_storage.rs
    ↓ OPFS API (web_sys)
Browser Origin Private File System
```

### Reading from Vault with Identity

```
JavaScript (with password)
    ↓
facades/wasm/vault.rs::vault_identity_from_passphrase()
    ↓ Platform::new()
domain/authentication/operations.rs::identity_from_passphrase()
    ↓ Get salt from vault
    ↓ platform.kdf() (KeyDerivationPort)
adapters/shared/argon2_kdf.rs
    ↓ Argon2 hash
    ↓ Return seed
    ↓ platform.identity() (IdentityPort)
adapters/shared/age_identity.rs
    ↓ Generate Age identity from seed
    ↓ Return private key string
JavaScript receives IdentityHandle
    ↓
facades/wasm/vault.rs::read_from_vault()
    ↓
domain/vault/operations.rs::read_namespace()
    ↓ Load vault + namespace
    ↓ Check expiration
    ↓ platform.encryption() (EncryptionPort)
adapters/shared/age_encryption.rs
    ↓ Age decrypt
    ↓ Return plaintext
JavaScript receives data
```

### WebAuthn PRF Flow

```
JavaScript calls webauthn/derive_key_with_prf()
    ↓
facades/wasm/webauthn/webauthn.rs
    ↓ navigator.credentials.get()
    ↓ Browser WebAuthn prompt
    ↓ User authenticates (biometrics/PIN)
    ↓ Get PRF outputs (first, second)
    ↓ platform.prf() (PrfPort)
adapters/wasm/webauthn_prf.rs
    ↓ HKDF key derivation
    ↓ Return 32-byte key
    ↓ Can be used as seed for Age identity
JavaScript receives derived key
```

## Key Design Decisions

### 1. Namespace-per-File Storage
**Decision**: Each namespace stored in separate `.ns` file
**Rationale**:
- Enables partial updates without rewriting entire vault
- Reduces lock contention on concurrent operations
- Simplifies deletion (just remove file)
**Trade-off**: More files, but OPFS handles this well

### 2. Separation of Metadata and Data
**Decision**: Vault metadata in separate `metadata.json`
**Rationale**:
- Can list vaults without reading all namespace data
- Metadata operations don't require namespace access
- Faster vault discovery and sync

### 3. Age Encryption with Multiple Recipients
**Decision**: Support multiple recipient public keys
**Rationale**:
- Enables secure sharing (vault owner + other users)
- Age format supports this natively
- Each namespace can have different recipients
**Note**: Currently only single recipient used (identity's public key)

### 4. Expiration on Read (Lazy Cleanup)
**Decision**: Check expiration when reading, not on timer
**Rationale**:
- No background tasks needed (WASM limitation)
- Simpler implementation
- Storage remains until next access (acceptable trade-off)
**Enhancement**: Optional force cleanup available

### 5. Deterministic Identity Derivation
**Decision**: Passphrase + salt → Argon2 → seed → Age identity
**Rationale**:
- User only needs to remember passphrase
- Same passphrase always produces same identity
- Vault-specific salts prevent rainbow table attacks
**Security**: Argon2 parameters chosen for browser performance vs security balance

### 6. Custom Serialization Format
**Decision**: Binary format with magic number for import/export
**Rationale**:
- Magic number prevents accidental corruption
- Version field enables future format evolution
- Binary more compact than JSON
**Format**: `[magic:4bytes][version:4bytes][data:rest]`

## Error Handling Strategy

### Three-Layer Error Handling

**Domain Errors** (`domain/*/error.rs`):
- Specific error types per domain module
- Rich context about business logic failures
- Implements `std::error::Error`

**Facade Conversion** (`facades/wasm/error_conversions.rs`):
- All domain errors → `JsValue` at boundary
- JavaScript receives error messages
- No Rust types exposed to JS

**Port Errors**:
- Some use `VaultError` (coupling issue)
- Some use `Box<dyn Error>` (generic)
- Conversions happen at adapter boundaries

### Error Context Preservation

**WASM Adapters**: Preserve full error context (stack traces from JS)
**Native Adapters**: Currently lose context (improvement opportunity)

## Performance Characteristics

### Zero-Cost Abstractions
- All adapters are zero-sized types (ZST)
- Platform is `Copy` (no heap allocation)
- Port trait methods use static dispatch internally
- Dynamic dispatch only at Platform accessors (`&dyn Trait`)

### WASM Bundle Size
- Age encryption: ~150KB (wasm-opt applied)
- Argon2: Memory-bound, uses WASM linear memory
- Total WASM binary: ~500KB (gzipped)

### Crypto Performance
- **Argon2**: ~100-300ms on modern browsers (depends on parameters)
- **Age encryption**: ~10-50ms per operation (file size dependent)
- **WebAuthn PRF**: ~1-3s (user interaction required)

### Storage Performance
- **OPFS**: Near-native filesystem performance
- **Concurrent operations**: Web Locks API prevents race conditions
- **Namespace isolation**: Enables parallel reads (different namespaces)

## Quick Reference

### Port Definition Checklist
- Trait with `Send + Sync` bounds
- Methods take `&self`
- Use borrowed types (`&str`, `&[u8]`)
- Async methods use `async_trait(?Send)`

### Adapter Implementation Checklist
- `#[derive(Clone, Copy)]` (zero-sized type)
- Simple `pub fn new() -> Self` constructor
- Implements corresponding port trait
- WASM: use web_sys/js_sys APIs
- Native: simple stub/mock

### Adding New Port
1. Create port trait in `src/ports/[name].rs`
2. Implement WASM adapter in `src/adapters/wasm/[name].rs`
3. Implement native stub in `src/adapters/native/[name].rs`
4. Export from respective `mod.rs` files with `#[cfg]`
5. Add to Platform struct and implement accessor
6. Write tests (1 platform + 3-5 per adapter)
7. Test: `cargo test --lib` and `wasm-pack test --headless --chrome`

### Build Commands
- `cargo check --lib` - Check native compilation
- `cargo test --lib` - Run native unit tests
- `wasm-pack build --target web` - Build WASM module
- `wasm-pack test --headless --chrome` - Run WASM tests
