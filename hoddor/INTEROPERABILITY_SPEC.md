# Hoddor Interoperability Specification

**WASM ↔ Native Migration via Hexagonal Architecture**

- **Version:** 1.2
- **Status:** In Progress (Phase 1: 3/5 ports complete - Logger ✅, Clock ✅, Persistence ✅)

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

| Port              | Responsibility                       | Used by          | Status |
| ----------------- | ------------------------------------ | ---------------- | ------ |
| `LoggerPort`      | Logging (info, warn, error, time)    | All modules      | ✅ Done |
| `ClockPort`       | Timestamps, performance measurement  | Vault, Sync      | ✅ Done |
| `PersistencePort` | Storage persistence check/request    | Persistence      | ✅ Done |
| `LockPort`        | Exclusive lock acquisition/release   | Vault, Sync      | ✅ Done |
| `StoragePort`     | File read/write, directory mgmt      | Vault            | 🔄 Next |
| `NotifierPort`    | Event notifications                  | Events           | Later  |

### Adapters (Implementation)

**WASM Adapters** (browser):

| Adapter           | Implements        | Technology                    | Status |
| ----------------- | ----------------- | ----------------------------- | ------ |
| `ConsoleLogger`   | LoggerPort        | Console API (FFI)             | ✅ Done |
| `Clock`           | ClockPort         | Performance API               | ✅ Done |
| `Persistence`     | PersistencePort   | Storage Manager API           | ✅ Done |
| `Locks`           | LockPort          | Web Locks API                 | ✅ Done |
| `OPFSStorage`     | StoragePort       | File System Access API (OPFS) | Next   |

**Native Adapters** (server):

| Adapter           | Implements        | Technology                 | Status |
| ----------------- | ----------------- | -------------------------- | ------ |
| `ConsoleLogger`   | LoggerPort        | stdout/stderr              | ✅ Done |
| `Clock`           | ClockPort         | std::time::SystemTime      | ✅ Done |
| `Persistence`     | PersistencePort   | No-op (always persistent)  | ✅ Done |
| `Locks`           | LockPort          | Stub (no-op)               | ✅ Done |
| `FsStorage`       | StoragePort       | tokio::fs / std::fs        | Next   |

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
- ALWAYS: `Send + Sync` bounds (async and multi-threading compatibility)
- ALWAYS: `&self` methods (stateless or internal mutability)
- PREFER: Borrowed types (`&str`, `&[u8]`) over owned
- ALWAYS: `#[derive(Clone, Copy)]` on adapters (zero-sized types)

### Platform Pattern (Dependency Injection)
```rust
// src/platform.rs
use crate::adapters::{Clock, ConsoleLogger, Persistence};
use crate::ports::{ClockPort, LoggerPort, PersistencePort};

#[derive(Clone, Copy)]
pub struct Platform {
    clock: Clock,
    logger: ConsoleLogger,
    persistence: Persistence,
}

impl Platform {
    pub fn new() -> Self {
        Self {
            clock: Clock::new(),
            logger: ConsoleLogger::new(),
            persistence: Persistence::new(),
        }
    }

    pub fn clock(&self) -> &dyn ClockPort { &self.clock }
    pub fn logger(&self) -> &dyn LoggerPort { &self.logger }
    pub fn persistence(&self) -> &dyn PersistencePort { &self.persistence }
}
```

**Key points**:
- Platform stores concrete adapter instances (not static references)
- Returns `&dyn Trait` from accessors (enables trait methods)
- `Copy` enables easy use in closures
- Zero-cost: all adapters are zero-sized types (ZSTs)

### Usage Pattern
```rust
use crate::Platform;

pub async fn business_logic() {
    let platform = Platform::new();
    platform.logger().log("Operation started");
}

// In structs
pub struct MyService {
    platform: Platform,
}

impl MyService {
    pub fn new() -> Self {
        Self { platform: Platform::new() }
    }

    pub fn method(&self) {
        self.platform.logger().log("Processing...");
    }
}
```

---

## 4. Migration Progress

### ✅ Completed Ports

#### LoggerPort (Step 1)

- **Port**: `LoggerPort` trait with 5 methods (log, error, warn, time, time_end)
- **WASM Adapter**: `ConsoleLogger` with integrated FFI bindings
- **Native Adapter**: `ConsoleLogger` with stdout/stderr
- 13 files updated, 199+ call sites migrated
- Zero performance regression

#### ClockPort (Step 2)

- **Port**: `ClockPort` trait with 2 methods (now, is_available)
- **WASM Adapter**: `Clock` using Performance API
- **Native Adapter**: `Clock` using SystemTime (Unix milliseconds)
- Tests: 5 native tests + 4 WASM tests
- Zero performance regression

#### PersistencePort (Step 3)

- **Port**: `PersistencePort` trait with 3 async methods (check, request, has_requested)
- **WASM Adapter**: `Persistence` using Storage Manager API
- **Native Adapter**: `Persistence` (always returns true, no-op)
- Added `async-trait` dependency for async trait methods
- Tests: 3 native tests + 3 WASM tests
- Zero performance regression

#### LockPort (Step 4)

- **Port**: `LockPort` trait with async `acquire()` method returning `LockGuard`
- **WASM Adapter**: `Locks` using Web Locks API with retry logic, exponential backoff
- **Native Adapter**: `Locks` (stub, always succeeds immediately)
- Migrated 3 call sites in vault.rs
- Tests: 4 native tests + 1 platform test
- Zero performance regression
- RAII pattern: lock released automatically on guard drop

### 🔧 Architecture Refactor

**Removed lazy_static pattern** - Simplified architecture:
- ✅ Removed 3 `global_*.rs` files (clock, logger, persistence)
- ✅ Removed `lazy_static` dependency from Cargo.toml
- ✅ Platform now stores concrete instances directly
- ✅ All adapters have `#[derive(Clone, Copy)]`
- ✅ Platform is `Copy` (zero-cost in closures)

**Current Architecture**:
```
src/
  ports/
    mod.rs
    logger.rs                        # LoggerPort trait
    clock.rs                         # ClockPort trait
    persistence.rs                   # PersistencePort trait
    lock.rs                          # LockPort trait
  adapters/
    mod.rs                           # Platform-specific exports
    wasm/
      mod.rs
      console_logger.rs              # WASM logger
      clock.rs                       # WASM clock (Performance API)
      persistence.rs                 # WASM persistence (Storage Manager)
      locks.rs                       # WASM locks (Web Locks API)
    native/
      mod.rs
      console_logger.rs              # Native logger (stdout/stderr)
      clock.rs                       # Native clock (SystemTime)
      persistence.rs                 # Native persistence (no-op)
      locks.rs                       # Native locks (stub)
  platform.rs                        # DI container
```

### 🔄 Next Steps (Priority Order)

1. **StoragePort** (file_system.rs)
   - Complex: Many functions, OPFS API
   - WASM: File System Access API
   - Native: `tokio::fs` / `std::fs`

2. **Domain Extraction** (vault.rs)
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

| Module          | WASM Adapter            | Native Adapter          | Status  |
| --------------- | ----------------------- | ----------------------- | ------- |
| `console.rs`    | `wasm/console_logger`   | `native/console_logger` | ✅ Done |
| `measure.rs`    | `wasm/clock`            | `native/clock`          | ✅ Done |
| `persistence.rs`| `wasm/persistence`      | `native/persistence`    | ✅ Done |
| `lock.rs`       | `wasm/locks`            | `native/locks`          | ✅ Done |
| `file_system.rs`| `wasm/opfs_storage`     | `native/fs_storage`     | 🔄 Next |

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
| Port            | Effort | Duration | Status      |
| --------------- | ------ | -------- | ----------- |
| LoggerPort      | Low    | 1 day    | ✅ Complete |
| ClockPort       | Low    | 1 day    | ✅ Complete |
| PersistencePort | Low    | 1 day    | ✅ Complete |
| LockPort        | Med    | 1 day    | ✅ Complete |
| StoragePort     | High   | 1 week   | 🔄 Next     |

**Progress**: 4/5 ports complete (80%)
**Time spent**: ~4 days
**Remaining**: ~1 week

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

### Dependency Injection via Platform
**Decision**: Platform struct stores concrete adapter instances (not lazy_static)
**Reason**: Simpler code, no runtime initialization, zero-cost abstraction
**Pattern**: All adapters are `Copy`, Platform is `Copy`
**Benefits**:
- No lazy_static dependency
- Fewer files (no global_*.rs)
- Easy to use in closures
- Zero runtime cost (all ZST)

### Trait Object Return from Platform
**Decision**: Platform accessors return `&dyn Trait` instead of concrete types
**Reason**: Enables trait methods without importing trait at call sites
**Benefit**: Cleaner API, better encapsulation

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

**Last Updated**: 2025-10-15 (4 ports complete - Logger, Clock, Persistence, Lock)
