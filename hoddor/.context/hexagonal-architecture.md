# Hoddor Hexagonal Architecture - AI Context

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
- PATTERN: Native can be simple stub initially (stdout/stderr for logger)

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

## Current Implementation Status

### Completed Ports
- **LoggerPort** (`ports/logger.rs`)
  - WASM: ConsoleLogger (browser console FFI)
  - Native: ConsoleLogger (stdout/stderr)
  - Migrated: 13 files, 199+ call sites
  - Global: `adapters::logger()`

### Next Migration Candidates (Priority)
1. **ClockPort** (measure.rs) - Simple, get_performance/now functions
2. **PersistencePort** (persistence.rs) - 3 functions, localStorage API
3. **LockPort** (lock.rs) - Medium complexity, retry logic
4. **StoragePort** (file_system.rs) - Complex, OPFS API, many functions
5. **Domain extraction** (vault.rs) - Most complex, business logic separation

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

### Simple Stub Example (Logger)
```rust
// src/adapters/native/console_logger.rs
pub struct ConsoleLogger;

impl LoggerPort for ConsoleLogger {
    fn log(&self, message: &str) {
        println!("[LOG] {}", message);
    }
    // ... other methods
}
```

Native can be:
- Simple stub (stdout/stderr)
- Full implementation (tracing crate)
- Decided later (YAGNI for now)

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

## Testing Pattern

### Port Contract Test
```rust
fn test_contract<T: PortTrait>(adapter: &T) {
    assert!(adapter.operation("test").is_ok());
}

#[cfg(all(test, target_arch = "wasm32"))]
#[wasm_bindgen_test]
fn test_wasm() {
    test_contract(&WasmAdapter::new());
}
```

## Critical Constraints

### Binary Compatibility
- Domain output MUST be byte-identical across platforms
- Vault format MUST match exactly
- Test: Create in WASM → Read in Native (both directions)

### No Regression
- WASM performance: max 5% degradation
- Benchmark before/after each port
- Reject if performance loss

### Progressive Migration
- One module at a time
- Old code continues working
- Test after each port addition

## Quick Reference

### When creating port:
`Send + Sync + &self + borrowed types`

### When using port:
`use adapters::instance;` (no trait import needed)

### When adding native:
`Create native/adapter.rs + add cfg to global_*.rs`

### Compilation check:
`cargo check --target wasm32-unknown-unknown`
