# Domain Extraction Strategy

**Phase 2 of Hexagonal Architecture Migration**

- **Version:** 2.0
- **Status:** Planning
- **Previous Phase:** Infrastructure Ports Complete (see INTEROPERABILITY_SPEC.md)

---

## 1. Context & Objectives

### Current State (Post-Phase 1)

**Infrastructure Layer** ✅ Complete:
- All I/O operations abstracted through 6 ports (Logger, Clock, Persistence, Lock, Storage, Notifier)
- Zero conditional compilation in business logic
- Platform-independent infrastructure access via `Platform` struct
- `vault.rs` uses only ports (no direct OPFS/FileSystem calls)

**Business Logic** ⚠️ Mixed:
- Located in: `vault.rs` (1306 lines), `crypto.rs` (350 lines), `sync.rs` (170 lines)
- Mixed with: API layer (#[wasm_bindgen]), infrastructure concerns
- Challenge: Hard to test without WASM, not reusable for native server

### Phase 2 Goals

**Extract Pure Domain Logic**:
1. Separate business rules from infrastructure
2. Make domain 100% testable without WASM
3. Enable code reuse for native server
4. Clear separation: Domain → Ports → Adapters

**Success Criteria**:
- Domain modules are pure Rust (no wasm_bindgen in domain/)
- All business logic testable with mocked ports
- vault.rs becomes a thin API layer
- Domain code portable to native server

---

## 2. Domain Structure

### Target Architecture

```
src/
├── domain/              # Pure business logic (portable)
│   ├── vault/          # Vault domain
│   ├── crypto/         # Cryptography domain
│   └── sync/           # Sync domain
├── ports/              # Interfaces (already done ✅)
├── adapters/           # Platform implementations (already done ✅)
├── platform.rs         # DI container (already done ✅)
└── vault.rs            # API Layer (wasm_bindgen) - to be simplified
```

### Domain Principles

**What Goes in Domain:**
- ✅ Business rules and validation logic
- ✅ Data structures (types, enums)
- ✅ Pure functions and algorithms
- ✅ Domain operations (CRUD, transformations)
- ✅ Business invariants enforcement

**What Stays Outside Domain:**
- ❌ wasm_bindgen bindings
- ❌ JsValue conversions
- ❌ Direct I/O operations (use ports via Platform)
- ❌ Platform-specific code

---

## 3. Domain: Vault

### Overview

**Current State**:
- File: `src/vault.rs` (1306 lines)
- Contains: API + Business Logic + Infrastructure calls
- Issues: Mixed concerns, hard to test, WASM-dependent

**Target State**:
- Domain: `src/domain/vault/` (pure business logic)
- API Layer: `src/vault.rs` (thin wasm_bindgen wrapper)

### Domain Vault Structure

```
src/domain/vault/
├── mod.rs              # Public domain API
├── types.rs            # Domain types
├── operations.rs       # Core vault operations
├── identity.rs         # Identity management
├── validation.rs       # Validation rules
└── expiration.rs       # Data expiration logic
```

### Module Responsibilities

#### types.rs - Domain Data Structures
**Purpose**: Define core domain types

**Contains**:
- `Vault` struct (metadata, identity_salts, username_pk, namespaces, sync_enabled)
- `VaultMetadata` (peer_id)
- `IdentitySalts` (salts, credential_ids)
- `NamespaceData` (data, expiration)
- `Expiration` (expires_at)

**Characteristics**:
- Pure data structures
- No I/O operations
- Serde serialization support
- Business invariants as methods

#### validation.rs - Validation Rules
**Purpose**: Enforce business rules

**Contains**:
- `validate_vault_name(name: &str)` → Check alphanumeric + underscore/hyphen
- `validate_namespace(namespace: &str)` → Check empty, invalid chars
- `validate_passphrase(passphrase: &str)` → Check empty/whitespace

**Characteristics**:
- Pure functions (no side effects)
- Return Result<(), VaultError>
- Reusable validation logic

#### operations.rs - Core Vault Operations
**Purpose**: Implement business operations using ports

**Contains**:
- `read_vault(storage: &dyn StoragePort, vault_name: &str)` → Load vault from storage
- `save_vault(platform: &Platform, vault_name: &str, vault: Vault)` → Persist vault
- `create_vault(vault_name: &str)` → Create new empty vault
- `delete_vault(storage: &dyn StoragePort, vault_name: &str)` → Remove vault
- `list_vaults(storage: &dyn StoragePort)` → List all vaults
- `export_vault(vault: &Vault)` → Serialize to binary format
- `import_vault(data: &[u8])` → Deserialize from binary format

**Pattern**: Operations accept Platform or specific ports as parameters

#### identity.rs - Identity Management
**Purpose**: Manage vault identities and salts

**Contains**:
- `generate_identity(passphrase: &str, salt: &[u8; 32])` → Create identity from passphrase
- `find_identity(vault: &Vault, passphrase: &str)` → Match passphrase to stored identity
- `add_identity(vault: &mut Vault, public_key: String, salt: [u8; 32])` → Store new identity
- `check_identity(vault: &Vault, identity: &IdentityHandle)` → Verify identity access

**Dependencies**: Uses crypto domain for key derivation

#### expiration.rs - Data Expiration
**Purpose**: Handle namespace expiration logic

**Contains**:
- `is_expired(expiration: &Option<Expiration>, now: i64)` → Check if data expired
- `cleanup_expired(platform: &Platform, vault: &mut Vault, vault_name: &str)` → Remove expired namespaces
- `calculate_expiration(expires_in_seconds: Option<i64>)` → Create Expiration

**Pattern**: Pure business logic for expiration rules

### Migration Strategy for Vault Domain

**Step 1: Extract Types** (Low Risk)
- Move struct definitions to `domain/vault/types.rs`
- No logic change, just relocation
- Update imports in vault.rs

**Step 2: Extract Validators** (Low Risk)
- Move validation functions to `domain/vault/validation.rs`
- Already pure functions, minimal changes
- Update call sites

**Step 3: Extract Operations** (Medium Risk)
- Create operation functions that accept Platform/ports as parameters
- Refactor vault.rs functions to use domain operations
- Pattern: `pub fn operation(platform: &Platform, ...) -> Result<T, VaultError>`

**Step 4: Simplify API Layer** (Final)
- vault.rs becomes thin wasm_bindgen wrappers
- Converts JsValue → Rust types
- Calls domain operations
- Converts Result → JsValue

---

## 4. Domain: Crypto

### Overview

**Current State**:
- File: `src/crypto.rs` (350 lines)
- Contains: Encryption, key derivation, identity management
- Issues: Some WASM bindings mixed with pure crypto

**Target State**:
- Domain: `src/domain/crypto/` (pure crypto logic)
- API Layer: Functions in crypto.rs remain for wasm_bindgen export

### Domain Crypto Structure

```
src/domain/crypto/
├── mod.rs              # Public domain API
├── encryption.rs       # Age encryption/decryption
├── keys.rs             # Key derivation (Argon2, HKDF)
└── identity.rs         # Identity management
```

### Module Responsibilities

#### encryption.rs - Age Encryption
**Purpose**: Encrypt/decrypt data using age

**Contains**:
- `encrypt_data(data: &[u8], recipients: &[Recipient])` → Encrypt with age
- `decrypt_data(encrypted: &[u8], identity: &Identity)` → Decrypt with identity
- Pure age operations, no WASM dependencies

#### keys.rs - Key Derivation
**Purpose**: Derive keys from passphrases

**Contains**:
- `derive_key_from_passphrase(passphrase: &str, salt: &[u8; 32])` → Argon2 KDF
- `derive_prf_key(prf_outputs: &[u8])` → HKDF for WebAuthn PRF
- Pure crypto algorithms

#### identity.rs - Identity Types
**Purpose**: Manage age identities

**Contains**:
- `IdentityHandle` wrapper
- Identity creation/serialization
- Public key extraction

### Migration Strategy for Crypto Domain

**Step 1: Identify Pure Functions**
- Scan crypto.rs for non-WASM functions
- Mark functions that only use standard crypto crates

**Step 2: Extract to Domain**
- Move pure crypto to domain/crypto/
- Keep wasm_bindgen exports in crypto.rs

**Step 3: Refactor Exports**
- Make crypto.rs call domain functions
- Thin wrappers for JS interop

---

## 5. Domain: Sync

### Overview

**Current State**:
- File: `src/sync.rs` (170 lines)
- Contains: Sync manager, operations, message types
- Issues: Coupled to WebRTC (WASM-only)

**Target State**:
- Domain: `src/domain/sync/` (sync logic)
- Keep WebRTC coupling in sync.rs for now (Phase 3 concern)

### Domain Sync Structure

```
src/domain/sync/
├── mod.rs              # Public domain API
├── types.rs            # Sync message types
└── operations.rs       # Operation logic
```

### Module Responsibilities

#### types.rs - Sync Types
**Purpose**: Define sync data structures

**Contains**:
- `OperationType` enum (Insert, Update, Delete)
- `SyncOperation` struct
- `SyncMessage` struct
- Pure data types, serializable

#### operations.rs - Sync Operations
**Purpose**: Sync business logic

**Contains**:
- `create_operation(namespace: String, op_type: OperationType, ...)` → Build operation
- `apply_operation(vault: &mut Vault, operation: SyncOperation)` → Apply to vault
- `merge_vault_state(current: &Vault, incoming: &SyncMessage)` → Merge logic

**Note**: WebRTC transport remains in sync.rs (not domain concern)

### Migration Strategy for Sync Domain

**Step 1: Extract Types**
- Move operation types to domain/sync/types.rs
- Keep sync manager in sync.rs (WebRTC-coupled)

**Step 2: Extract Operation Logic**
- Pure functions for creating/applying operations
- Testable without WebRTC

**Step 3: Later (Phase 3)**
- Abstract P2P transport (WebRTC vs libp2p)
- Create TransportPort if needed

---

## 6. Implementation Order

### Priority 1: Vault Domain (Highest Impact)
**Effort**: 2-3 days
**Files**: ~5 new files, 1 simplified
**Benefits**:
- 1000+ lines of business logic become testable
- Clear separation of concerns
- Foundation for native server

**Steps**:
1. Create domain/vault/ structure
2. Extract types.rs (Day 1 morning)
3. Extract validation.rs (Day 1 afternoon)
4. Extract operations.rs (Day 2)
5. Simplify vault.rs API layer (Day 3)

### Priority 2: Crypto Domain
**Effort**: 1 day
**Files**: ~3 new files
**Benefits**:
- Pure crypto logic reusable
- Easier to audit security
- Less coupling to WASM

**Steps**:
1. Identify pure crypto functions
2. Extract to domain/crypto/
3. Refactor crypto.rs to use domain

### Priority 3: Sync Domain
**Effort**: 1 day
**Files**: ~2 new files
**Benefits**:
- Sync logic testable
- Prepare for alternative transports

**Steps**:
1. Extract sync types
2. Extract operation logic
3. Keep WebRTC in sync.rs

---

## 7. Testing Strategy

### Domain Testing Principles

**Unit Tests for Domain**:
- Pure Rust tests (no wasm-bindgen-test)
- Mock ports when needed
- Test business rules in isolation
- Run with `cargo test`

**Example Test Structure**:
```rust
// tests/domain/vault/operations_tests.rs
#[cfg(test)]
mod tests {
    use super::*;

    // Mock storage for testing
    struct MockStorage { /* ... */ }
    impl StoragePort for MockStorage { /* ... */ }

    #[test]
    fn test_create_vault_valid_name() {
        // Test pure domain logic
    }
}
```

**Integration Tests**:
- Keep existing WASM tests for API layer
- Add native integration tests for domain
- Verify port contracts

---

## 8. API Layer Pattern

### Current Pattern (vault.rs)
```
#[wasm_bindgen]
pub async fn operation(params: JsValue) -> Result<JsValue, JsValue> {
    // JsValue conversion
    // Business logic (mixed)
    // Infrastructure calls (via Platform) ✅
    // Result conversion
}
```

### Target Pattern (vault.rs)
```
#[wasm_bindgen]
pub async fn operation(params: JsValue) -> Result<JsValue, JsValue> {
    // 1. Convert JsValue → Rust types
    // 2. Call domain::vault::operation(platform, ...)
    // 3. Convert Result → JsValue
}
```

**Benefits**:
- vault.rs becomes ~50% smaller
- Business logic in domain/ (testable, portable)
- Clear separation of concerns

---

## 9. File Organization

### Before (Current)
```
src/
├── vault.rs              # 1306 lines: API + Business + Infrastructure
├── crypto.rs             # 350 lines: Crypto + WASM bindings
├── sync.rs               # 170 lines: Sync + WebRTC
└── [6 ports + adapters]  # ✅ Done
```

### After (Phase 2 Complete)
```
src/
├── domain/               # NEW: Pure business logic
│   ├── vault/           # ~800 lines (extracted from vault.rs)
│   ├── crypto/          # ~200 lines (extracted from crypto.rs)
│   └── sync/            # ~100 lines (extracted from sync.rs)
├── vault.rs             # ~500 lines (API layer only)
├── crypto.rs            # ~150 lines (WASM exports)
├── sync.rs              # ~70 lines (WebRTC transport)
└── [ports + adapters]   # ✅ Unchanged
```

**Net Result**:
- ~1100 lines of pure domain logic
- Fully testable without WASM
- Reusable for native server

---

## 10. Non-Goals (Out of Scope)

**Not in Phase 2**:
- ❌ WebRTC abstraction (Phase 3 - P2P transport)
- ❌ WebAuthn abstraction (WASM-only feature)
- ❌ Native server implementation (Phase 3)
- ❌ Additional infrastructure ports (Phase 1 complete)

**Keep As-Is**:
- `webrtc.rs` - WASM-only, complex, low priority
- `webauthn/` - WASM-only, specialized use case
- `signaling.rs` - WebRTC-specific
- `global.rs` - Used only by WASM adapters (correct placement)
- `measure.rs` - Already uses Platform correctly
- `notifications.rs` - Pure data types (no logic)

---

## 11. Success Metrics

### Quantitative Metrics
- ✅ Domain code has 0 wasm_bindgen imports
- ✅ Domain tests run with `cargo test` (no --target wasm32)
- ✅ vault.rs reduced by ~40-50% (from 1306 to ~600-700 lines)
- ✅ 80%+ domain code coverage with pure Rust tests

### Qualitative Metrics
- ✅ Clear separation: Domain → Ports → Adapters
- ✅ Business logic readable and maintainable
- ✅ Domain code reusable for native server
- ✅ vault.rs is a thin API layer (easy to understand)

---

## 12. Risk Mitigation

### Risk 1: Breaking Existing Tests
**Mitigation**:
- Extract incrementally (types → validators → operations)
- Keep existing WASM tests running at each step
- Don't modify behavior, only structure

### Risk 2: Increased Complexity
**Mitigation**:
- Clear documentation of domain boundaries
- Consistent patterns across domains
- Simpler vault.rs offsets domain/ complexity

### Risk 3: Over-Engineering
**Mitigation**:
- Extract only proven business logic
- Don't create abstractions speculatively
- Follow actual code reuse needs

---

## 13. Next Steps

### Immediate Actions
1. Review this document with team
2. Validate domain boundaries
3. Start with Vault domain extraction (highest value)

### Implementation Phases
- **Week 1**: Vault domain extraction
- **Week 2**: Crypto domain extraction
- **Week 3**: Sync domain extraction
- **Week 4**: Testing, documentation, refinement

---

## 14. References

### Related Documents
- `INTEROPERABILITY_SPEC.md` - Phase 1 (Infrastructure Ports)
- `.context/hexagonal-architecture.md` - Architecture principles

### Key Concepts
- **Domain-Driven Design**: Business logic isolation
- **Hexagonal Architecture**: Ports & Adapters pattern
- **Clean Architecture**: Dependency inversion

---

**Living Document**: Updated as domain extraction progresses.

**Last Updated**: 2025-10-15 (Planning Phase)
