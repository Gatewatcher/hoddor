# Hexagonal Architecture - AI Context

## Architecture Type
Hexagonal (Ports & Adapters) - Supports WASM + Native with shared business logic

## Directory Map
```
src/
  ports/           # Trait definitions (interfaces)
  adapters/        # Platform implementations
    global_*.rs    # Singletons with #[cfg]
    wasm/          # Browser implementations
    native/        # Native implementations (create when needed)
  domain/          # Pure business logic (future)
  [modules]/       # Existing code (progressive migration)
```

## Core Patterns

### Pattern: Port Definition
```rust
// @location: src/ports/[name].rs
// @required: Send + Sync, &self, borrowed types
pub trait PortName: Send + Sync {
    fn operation(&self, input: &str) -> Result<Output, Error>;
}
```

**Rules:**
- ALWAYS: `Send + Sync` bounds (async/lazy_static/tokio compatibility)
- ALWAYS: `&self` methods (stateless or internal mutability)
- PREFER: Borrowed types (`&str`, `&[u8]`) over owned
- SINGLE: One responsibility per port

### Pattern: WASM Adapter
```rust
// @location: src/adapters/wasm/[name].rs
use crate::ports::PortName;

pub struct WasmAdapter;

impl WasmAdapter {
    pub fn new() -> Self { Self }
}

impl PortName for WasmAdapter {
    fn operation(&self, input: &str) -> Result<Output, Error> {
        // Use web_sys, js_sys, or existing FFI
    }
}
```

### Pattern: Global Singleton
```rust
// @location: src/adapters/global_[name].rs
use crate::ports::PortName;
use lazy_static::lazy_static;

#[cfg(target_arch = "wasm32")]
use crate::adapters::wasm::WasmAdapter;
#[cfg(not(target_arch = "wasm32"))]
use crate::adapters::native::NativeAdapter;

lazy_static! {
    pub static ref INSTANCE: WasmAdapter = WasmAdapter::new();
    // OR: pub static ref INSTANCE: NativeAdapter = NativeAdapter::new();
}

/// Returns trait object - no trait import needed at call sites
pub fn instance() -> &'static dyn PortName {
    &*INSTANCE
}
```

**Key points:**
- Single lazy_static (cfg selects type at compile time)
- Returns `&'static dyn Trait` (trait object)
- No need to import trait at usage sites

### Pattern: Module Exports
```rust
// @location: src/ports/mod.rs
pub mod port_name;
pub use port_name::PortName;

// @location: src/adapters/mod.rs
pub mod global_name;
pub use global_name::{instance, INSTANCE};
```

### Pattern: Usage in Code
```rust
// @required: Only import adapter accessor (trait object pattern)
use crate::adapters::instance;

pub async fn business_logic() {
    instance().operation("data")?;  // Works directly - no trait import needed
}
```

**Why no trait import?** We return `&'static dyn Trait`, so methods are available directly.

## Decision Rules

### RULE: Global vs Injection
- USE: Global singleton via `lazy_static`
- REASON: Non-invasive, zero-cost, matches codebase
- EXCEPTION: None currently

### RULE: Target Selection
- USE: `#[cfg(target_arch = "wasm32")]` for WASM
- USE: `#[cfg(not(target_arch = "wasm32"))]` for native
- REASON: Compile-time, zero-cost, type-safe

### RULE: Dual Implementation
- CREATE: Both WASM and native adapters together
- REASON: Avoids rust-analyzer "inactive code" warnings
- PATTERN: Native can be simple stub initially (minimal implementation)

### RULE: Send + Sync
- ALWAYS: Include on all port traits
- REASON: Required for lazy_static (Sync), async (Send), tokio
- COST: Zero (compile-time only)

### RULE: Trait Object Return
- PATTERN: Functions return `&'static dyn Trait`
- BENEFIT: No trait import needed at call sites
- BENEFIT: Rust-analyzer sees all code as active

## Migration Checklist

### Adding New Port
- [ ] Create `src/ports/[name].rs` with trait (Send + Sync, &self, borrowed types)
- [ ] Create `src/adapters/wasm/[name].rs` with WASM implementation
- [ ] Create `src/adapters/native/[name].rs` with native implementation (can be stub)
- [ ] Create `src/adapters/global_[name].rs` returning `&'static dyn Trait`
- [ ] Export from `src/ports/mod.rs`
- [ ] Export from `src/adapters/mod.rs` with #[cfg] for wasm/native modules
- [ ] Migrate usage sites: replace direct calls with `instance().method()`
- [ ] Test both targets: `cargo check` and `cargo check --target wasm32-unknown-unknown`

### Migration Transform
```rust
// BEFORE
use crate::platform_module::function;
function("arg");

// AFTER
use crate::adapters::instance;
instance().method("arg");  // No trait import needed!
```

## Pattern: Platform (Dependency Injection Container)

**Purpose:** Store port references in structs instead of calling globals repeatedly.

```rust
// @location: src/platform.rs
#[derive(Clone, Copy)]
pub struct Platform {
    port1: &'static dyn Port1Trait,
    port2: &'static dyn Port2Trait,
}

impl Platform {
    pub fn new() -> Self {
        Self {
            port1: crate::adapters::port1(),
            port2: crate::adapters::port2(),
        }
    }

    pub fn port1(&self) -> &'static dyn Port1Trait {
        self.port1
    }

    pub fn port2(&self) -> &'static dyn Port2Trait {
        self.port2
    }
}
```

**Usage Pattern:**

```rust
// 1. Store in struct
pub struct MyStruct {
    platform: Platform,  // Store once
    // ... other fields
}

impl MyStruct {
    pub fn new() -> Self {
        Self {
            platform: Platform::new(),  // Initialize once
            // ...
        }
    }

    pub fn method(&self) {
        self.platform.port1().operation("data");  // Use stored Platform
    }

    // For closures: Platform is Copy
    pub fn with_closure(&self) {
        let platform = self.platform;  // Copy for closure
        let callback = move || {
            platform.port1().operation("data");
        };
    }
}
```

**WASM Entry Points Pattern:**

```rust
#[wasm_bindgen]
pub async fn entry_point(arg: &str) -> Result<T, JsValue> {
    let platform = Platform::new();  // Create once per entry
    internal_logic(&platform, arg).await
}

async fn internal_logic(platform: &Platform, arg: &str) -> Result<T, JsValue> {
    platform.port1().operation("processing");
    // Business logic
}
```

**Decision Tree:**

```
Does struct have lifetime/state?
├─ YES → Store Platform field + initialize in constructor
│
└─ NO → Use dual-layer pattern (Platform::new() in entry point)
          For stateless WASM entry points or standalone functions
```

**When to use:**
- ✅ Structs with methods → store as field
- ✅ WASM entry points → create once, pass to internal
- ✅ Closures → copy Platform (it's Copy)
- ❌ Don't call Platform::new() in loops
- ❌ Don't pass Platform through many layers (store in struct instead)

**Migration:**
```rust
// BEFORE (global singleton)
use crate::adapters::instance;
instance().operation("msg");

// AFTER (Platform in struct)
pub struct Foo {
    platform: Platform,
}
self.platform.port().operation("msg");
```

## Anti-Patterns

### ❌ Forgetting Native Implementation
```rust
// WRONG - rust-analyzer will complain
#[cfg(target_arch = "wasm32")]
pub mod wasm;
// Missing: native implementation!

// CORRECT - create both
#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
```

### ❌ Returning Concrete Type Instead of Trait Object
```rust
// WRONG - requires trait import at call sites
pub fn instance() -> &'static WasmAdapter { ... }

// CORRECT - trait object works without import
pub fn instance() -> &'static dyn PortTrait { ... }
```

### ❌ Missing Trait Import (Old Pattern - No Longer Needed)
```rust
// WRONG - won't compile
use crate::adapters::instance;
instance().method("arg");  // Error: method not found

// CORRECT
use crate::adapters::instance;
use crate::ports::PortTrait;  // Required!
instance().method("arg");
```

### ❌ Platform Code in Domain
```rust
// DON'T import in domain/
use web_sys::*;  // Never in domain
use tokio::*;    // Never in domain
```

### ❌ Skipping Send + Sync
```rust
// WRONG
pub trait BadPort {
    fn method(&self);
}

// CORRECT
pub trait GoodPort: Send + Sync {
    fn method(&self);
}
```

## Native Implementation Pattern

### Simple Stub Example
```rust
// src/adapters/native/[name].rs
pub struct NativeAdapter;

impl NativeAdapter {
    pub fn new() -> Self { Self }
}

impl PortName for NativeAdapter {
    fn operation(&self, input: &str) -> Result<Output, Error> {
        // Simple implementation using std lib
        Ok(output)
    }
}
```

Native implementations can be:
- Simple stub (minimal functionality)
- Full implementation (complete with external crates)
- Decided later (YAGNI approach)

### Dependencies Pattern
```toml
[dependencies]
lazy_static = "1.5"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3" }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1", features = ["full"] }
```

## Testing Strategy

### Three-Level Testing Hierarchy

**PRINCIPLE:** Avoid redundancy - each level tests different concerns.

```
Level 1: Platform (src/platform.rs)
  → Integration: Interface accessibility only (1 test per port)

Level 2: WASM Adapter (src/adapters/wasm/[name].rs)
  → Unit: WASM-specific implementation + behaviors (3-5 tests)

Level 3: Native Adapter (src/adapters/native/[name].rs)
  → Unit: Native-specific implementation + behaviors (3-5 tests)
```

### Test Responsibilities by Level

**Platform (Integration) - src/platform.rs**
- ✅ Port accessible via Platform
- ✅ Basic call doesn't panic
- ❌ NO detailed behaviors
- ❌ NO edge cases
- **Target:** ONE simple test per port

**WASM Adapter (Unit) - src/adapters/wasm/[name].rs**
- ✅ All port methods work correctly
- ✅ WASM-specific behaviors (web_sys, js_sys APIs)
- ✅ Edge cases, precision, validation
- **Patterns:**
  - Module: `#[cfg(all(test, target_arch = "wasm32"))]`
  - Tests: `#[wasm_bindgen_test]`
  - Configure: `wasm_bindgen_test_configure!(run_in_browser);`
  - **Important:** Tests only run in WASM (module not compiled in Native)

**Native Adapter (Unit) - src/adapters/native/[name].rs**
- ✅ All port methods work correctly
- ✅ Native-specific behaviors (std lib, tokio)
- ✅ Platform guarantees, real delays (thread::sleep)
- **Patterns:**
  - Standard `#[test]` attributes
  - Test native-specific functionality

### Test Execution

```bash
cargo test --lib                               # Native unit tests
wasm-pack test --headless --chrome            # WASM tests (unit + integration)
cargo test --lib adapters::native::[name]::tests   # Specific adapter
```

### Checklist for New Port Tests

1. **Platform:** ONE integration test (access + no panic)
2. **WASM:** 3-5 unit tests (implementation + WASM-specific)
3. **Native:** 3-5 unit tests (implementation + Native-specific)
4. **Verify:** Run both `cargo test --lib` and `wasm-pack test --headless --chrome`

### Anti-Patterns

❌ **Redundant tests across levels** - Same behavior tested in Platform AND adapters
❌ **Detailed testing in Platform** - Implementation details belong in adapters
❌ **Missing `wasm_bindgen_test_configure`** - WASM tests won't run in browser
❌ **Testing same edge cases in both adapters** - Only test platform-specific behaviors

### Expected Coverage per Port

- Platform: 1 test (integration)
- WASM Adapter: 3-5 tests (unit + WASM-specific)
- Native Adapter: 3-5 tests (unit + Native-specific)
- **Target: 7-11 tests total per port, zero redundancy**

## Critical Constraints

### Binary Compatibility
- Domain output MUST be byte-identical across platforms
- Data format MUST match exactly between WASM and Native
- Test: Create data in WASM → Read in Native (and reverse)

### No Regression
- Performance: max 5% degradation acceptable
- Benchmark before/after each port migration
- Reject changes causing performance loss

### Progressive Migration
- Migrate one port at a time
- Existing code continues working during migration
- Test after each port addition
- Maintain backward compatibility

## Quick Reference

### When creating port:
`Send + Sync + &self + borrowed types`

### When using port:
`use adapters::instance;` (no trait import needed)

### When adding native:
`Create native/adapter.rs + add cfg to global_*.rs`

### Compilation check:
`cargo check --target wasm32-unknown-unknown`
