# Hoddor Interoperability Specification

**WASM ↔ Native Migration via Hexagonal Architecture**

- **Version:** 1.1
- **Status:** In Progress (Step 1 - LoggerPort ✅ Complete)

---

## 1. Context & Goals

### Current State
Hoddor is a browser-based vault built in Rust, compiled exclusively for WASM:
- **Target**: `wasm32-unknown-unknown` via `wasm_bindgen`
- **Storage**: Browser Origin Private File System (OPFS)
- **APIs**: Web Locks, Performance, Console APIs
- **P2P**: WebRTC for peer communication
- **Crypto**: age encryption, Argon2 key derivation

### Target State
Enable Hoddor to run on both platforms:
1. **Browser** (current): End-user clients with local vaults
2. **Server** (new): Native Rust backend as central hub for distributed P2P vault network

### Critical Constraints
- ✅ **Binary compatibility**: Vault format must be identical across platforms
- ✅ **No regression**: Existing WASM code must work without degradation
- ✅ **Maximum reuse**: Business logic (crypto, vault, sync) shared 100%
- ✅ **Progressive migration**: Testable incremental steps

---

## 2. Architecture: Hexagonal (Ports & Adapters)

### Separation of Concerns
```
┌─────────────────────────────────────┐
│        DOMAIN (Business Core)       │
│  - Vault logic                      │
│  - Cryptography                     │
│  - Sync rules                       │
│  - Binary format (invariant)        │
└─────────────────────────────────────┘
              ▲           ▲
              │           │
    ┌─────────┴───┐   ┌──┴──────────┐
    │   PORTS     │   │   PORTS     │
    │  (Input)    │   │  (Output)   │
    └─────────────┘   └─────────────┘
         ▲                   ▲
         │                   │
    ┌────┴────┐         ┌────┴─────┐
    │ ADAPTERS│         │ ADAPTERS │
    │ Primary │         │Secondary │
    └─────────┘         └──────────┘
```

**Benefits**:
- Domain totally **platform-independent**
- Adapters **interchangeable** without touching business logic
- **Testable**: Mock all ports
- **Extensible**: Easy to add new platforms (mobile, embedded, etc.)

### Port Definitions (Output)

| Port              | Responsibility                       | Used by          |
| ----------------- | ------------------------------------ | ---------------- |
| `LoggerPort` ✅   | Logging (info, warn, error, time)    | All modules      |
| `ClockPort`       | Timestamps, performance measurement  | Vault, Sync      |
| `PersistencePort` | Storage persistence check/request    | Persistence      |
| `LockPort`        | Exclusive lock acquisition/release   | Vault, Sync      |
| `StoragePort`     | File read/write, directory mgmt      | Vault            |
| `NotifierPort`    | Event notifications                  | Events           |

### Adapters (Implementation)

**WASM Adapters** (browser):

| Adapter           | Implements        | Technology                    |
| ----------------- | ----------------- | ----------------------------- |
| `ConsoleLogger`✅ | LoggerPort        | Console API (FFI)             |
| `PerformanceClock`| ClockPort         | Performance API               |
| `WebLocks`        | LockPort          | Web Locks API                 |
| `OPFSStorage`     | StoragePort       | File System Access API (OPFS) |
| `StorageManager`  | PersistencePort   | Storage Manager API           |

**Native Adapters** (server):

| Adapter           | Implements        | Technology                 |
| ----------------- | ----------------- | -------------------------- |
| `ConsoleLogger`✅ | LoggerPort        | stdout/stderr              |
| `StdClock`        | ClockPort         | std::time::Instant         |
| `TokioLocks`      | LockPort          | tokio::sync::Mutex/RwLock  |
| `FsStorage`       | StoragePort       | tokio::fs / std::fs        |
| `AlwaysPersistent`| PersistencePort   | No-op (always persistent)  |

---

## 3. Implementation Pattern

### Port Trait Definition
```rust
// src/ports/logger.rs
pub trait LoggerPort: Send + Sync {
    fn log(&self, message: &str);
    fn error(&self, message: &str);
    fn warn(&self, message: &str);
    fn time(&self, label: &str);
    fn time_end(&self, label: &str);
}
```

**Rules**:
- ALWAYS: `Send + Sync` bounds (async/lazy_static/tokio compatibility)
- ALWAYS: `&self` methods (stateless or internal mutability)
- PREFER: Borrowed types (`&str`, `&[u8]`) over owned

### Global Singleton Pattern
```rust
// src/adapters/global_logger.rs
use lazy_static::lazy_static;

#[cfg(target_arch = "wasm32")]
use crate::adapters::wasm::ConsoleLogger;
#[cfg(not(target_arch = "wasm32"))]
use crate::adapters::native::ConsoleLogger;

lazy_static! {
    pub static ref LOGGER: ConsoleLogger = ConsoleLogger::new();
}

pub fn logger() -> &'static dyn LoggerPort {
    &*LOGGER
}
```

**Key points**:
- Single `lazy_static` (cfg selects type at compile time)
- Returns `&'static dyn Trait` (trait object)
- **No trait import needed at call sites**

### Usage Pattern
```rust
use crate::adapters::logger;

pub async fn business_logic() {
    logger().log("Operation started");  // Works directly!
}
```

---

## 4. Migration Progress

### ✅ Completed: LoggerPort (Step 1)

**Implementation**:
- **Port**: `LoggerPort` trait with 5 methods (log, error, warn, time, time_end)
- **WASM Adapter**: `ConsoleLogger` with integrated FFI bindings
- **Native Adapter**: `ConsoleLogger` with stdout/stderr
- **Global**: `logger()` function returning `&'static dyn LoggerPort`

**Migration Stats**:
- 13 files updated (10 source + 3 tests)
- 199+ call sites migrated
- 1 file deleted (`console.rs` → integrated into adapter)
- Zero performance regression
- Both targets compile successfully

**Files Migrated**:
- `src/crypto.rs`, `src/vault.rs`, `src/webrtc.rs`, `src/signaling.rs`
- `src/sync.rs`, `src/file_system.rs`, `src/measure.rs` (time_it! macro)
- `src/webauthn/mod.rs`, `src/webauthn/webauthn.rs`
- `tests/benchmark.rs`, `tests/test_utils.rs`, `tests/vault_operations.rs`

**Architecture Created**:
```
src/
  ports/
    mod.rs
    logger.rs                        # LoggerPort trait
  adapters/
    mod.rs
    global_logger.rs                 # Global singleton
    wasm/
      mod.rs
      console_logger.rs              # WASM impl + FFI bindings
    native/
      mod.rs
      console_logger.rs              # Native impl (stdout/stderr)
```

### 🔄 Next Steps (Priority Order)

1. **ClockPort** (measure.rs)
   - Simple migration: `get_performance()`, `now()`
   - WASM: Performance API
   - Native: `std::time::Instant`

2. **PersistencePort** (persistence.rs)
   - 3 functions: check, request, has_requested
   - WASM: Storage Manager API
   - Native: No-op (always persistent)

3. **LockPort** (lock.rs)
   - Medium complexity: retry logic, timeout handling
   - WASM: Web Locks API
   - Native: `tokio::sync::Mutex`

4. **StoragePort** (file_system.rs)
   - Complex: Many functions, OPFS API
   - WASM: File System Access API
   - Native: `tokio::fs` / `std::fs`

5. **Domain Extraction** (vault.rs)
   - Most complex: Separate business logic from infrastructure
   - Extract pure domain logic
   - Inject all ports

---

## 5. Module Migration Strategy

### Core Modules (Business Logic)

| Module      | Destination      | Strategy                            | Difficulty |
| ----------- | ---------------- | ----------------------------------- | ---------- |
| `crypto.rs` | `domain/crypto`  | Direct extraction, 95% portable     | ⭐ Low     |
| `vault.rs`  | `domain/vault`   | Major refactor, inject all ports    | ⭐⭐⭐ High |
| `sync.rs`   | `domain/sync`    | Extract logic, separate from WebRTC | ⭐⭐ Med   |
| `errors.rs` | `domain/errors`  | Direct migration                    | ⭐ Low     |

### Infrastructure Modules (To Adapt)

| Module          | WASM Adapter            | Native Adapter          | Status |
| --------------- | ----------------------- | ----------------------- | ------ |
| `console.rs`    | `wasm/console_logger`   | `native/console_logger` | ✅ Done|
| `measure.rs`    | `wasm/perf_clock`       | `native/std_clock`      | Next   |
| `persistence.rs`| `wasm/storage_manager`  | `native/always_persist` | Next   |
| `lock.rs`       | `wasm/web_locks`        | `native/tokio_locks`    | Later  |
| `file_system.rs`| `wasm/opfs_storage`     | `native/fs_storage`     | Later  |

### Platform-Specific Modules (Keep Separate)

| Module          | Strategy                        | Justification                           |
| --------------- | ------------------------------- | --------------------------------------- |
| `webrtc.rs`     | Keep WASM-only initially        | Browser API, need native alt (libp2p)   |
| `webauthn/`     | Keep WASM-only                  | Browser auth, not priority for server   |
| `signaling.rs`  | Abstract later                  | Depends on P2P network strategy         |

---

## 6. Testing & Validation

### Test Strategy
- **Unit tests**: Domain with mocked ports (no external deps)
- **Integration tests**: Per-adapter contract verification
- **E2E tests**: Cross-platform compatibility (WASM vault ↔ Native vault)
- **Regression tests**: Performance benchmarks (< 5% degradation)

### Success Criteria
- ✅ Binary compatibility: WASM ↔ Native vaults 100% interoperable
- ✅ Performance: < 5% WASM regression
- ✅ Coverage: > 80% on domain layer
- ✅ Compilation: Both targets compile without warnings

---

## 7. Timeline & Effort

### Phase 1: Port Migrations (Current)
| Port            | Effort | Duration | Status     |
| --------------- | ------ | -------- | ---------- |
| LoggerPort      | Low    | 1 day    | ✅ Complete|
| ClockPort       | Low    | 1 day    | Next       |
| PersistencePort | Low    | 1 day    | Next       |
| LockPort        | Med    | 2-3 days | Later      |
| StoragePort     | High   | 1 week   | Later      |

**Estimated**: 2-3 weeks

### Phase 2: Domain Extraction
- Extract vault business logic: 2-3 weeks
- Remove platform dependencies
- Inject all ports

### Phase 3: Network & Advanced Features (Optional)
- libp2p integration: 2-3 weeks
- HTTP/gRPC API: 1 week
- Advanced CLI: 1 week

**Total MVP**: 6-8 weeks

---

## 8. Key Decisions & Patterns

### Dual Implementation Strategy
**Decision**: Create both WASM and native adapters together
**Reason**: Avoids rust-analyzer "inactive code" warnings
**Pattern**: Native can be simple stub initially (e.g., stdout for logger)

### Trait Object Return
**Decision**: Functions return `&'static dyn Trait` instead of concrete types
**Reason**: No trait import needed at call sites, cleaner API
**Benefit**: Rust-analyzer sees all code as active

### Global Singleton vs Injection
**Decision**: Use global singleton via `lazy_static`
**Reason**: Non-invasive, zero-cost, matches existing codebase style
**Future**: May add dependency injection for complex cases (Platform struct)

---

## 9. References

### Documentation
- Architecture context: `.context/hexagonal-architecture.md`
- Port definitions: `src/ports/`
- Adapter implementations: `src/adapters/`

### External Resources
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

---

**Living Document**: This specification is updated as the project progresses.

**Last Updated**: 2025-10-13 (LoggerPort migration complete)
