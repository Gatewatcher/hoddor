# Hexagonal Architecture - AI Context

## Architecture Type
Hexagonal (Ports & Adapters) - Supports WASM + Native with shared business logic

## Directory Map
```
src/
  ports/           # Trait definitions (interfaces)
  adapters/        # Platform implementations
    wasm/          # Browser implementations
    native/        # Native implementations
  platform.rs      # Dependency injection container
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
- ALWAYS: `Send + Sync` bounds (async and multi-threading compatibility)
- ALWAYS: `&self` methods (stateless or internal mutability)
- PREFER: Borrowed types (`&str`, `&[u8]`) over owned
- SINGLE: One responsibility per port

### Pattern: WASM Adapter
```rust
// @location: src/adapters/wasm/[name].rs
use crate::ports::PortName;

#[derive(Clone, Copy)]
pub struct Adapter;

impl Adapter {
    pub fn new() -> Self { Self }
}

impl PortName for Adapter {
    fn operation(&self, input: &str) -> Result<Output, Error> {
        // Use web_sys, js_sys, or existing FFI
    }
}
```

**Key points:**
- Always `#[derive(Clone, Copy)]` on adapters (zero-sized types)
- Simple `new()` constructor
- Implements the port trait

### Pattern: Native Adapter
```rust
// @location: src/adapters/native/[name].rs
use crate::ports::PortName;

#[derive(Clone, Copy)]
pub struct Adapter;

impl Adapter {
    pub fn new() -> Self { Self }
}

impl PortName for Adapter {
    fn operation(&self, input: &str) -> Result<Output, Error> {
        // Use std lib or external crates
    }
}
```

**Key points:**
- Same structure as WASM adapter
- Native-specific implementation (std, tokio, etc.)
- Can be simple stub initially

### Pattern: Module Exports
```rust
// @location: src/ports/mod.rs
pub mod port_name;
pub use port_name::PortName;

// @location: src/adapters/mod.rs
#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub use wasm::{Adapter1, Adapter2, Adapter3};
#[cfg(not(target_arch = "wasm32"))]
pub use native::{Adapter1, Adapter2, Adapter3};
```

**Key points:**
- Use `#[cfg]` to select platform at compile-time
- Export concrete adapter types (not trait objects)
- Same names across platforms for seamless switching

## Decision Rules

### RULE: Dependency Injection via Platform
- USE: Platform struct stores concrete adapter instances
- REASON: Zero-cost abstraction, no lazy initialization needed
- PATTERN: `Platform::new()` creates all adapters at once

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
- REASON: Required for async (Send), multi-threading safety
- COST: Zero (compile-time only)

### RULE: Clone + Copy Adapters
- ALWAYS: `#[derive(Clone, Copy)]` on all adapters
- REASON: Makes Platform `Copy`, enables easy use in closures
- REQUIREMENT: Adapters must be zero-sized types (ZST)

## Migration Checklist

### Adding New Port
- [ ] Create `src/ports/[name].rs` with trait (Send + Sync, &self, borrowed types)
- [ ] Create `src/adapters/wasm/[name].rs` with WASM implementation (`#[derive(Clone, Copy)]`)
- [ ] Create `src/adapters/native/[name].rs` with native implementation (`#[derive(Clone, Copy)]`)
- [ ] Export from `src/ports/mod.rs`
- [ ] Export from `src/adapters/mod.rs` with #[cfg] for both platforms
- [ ] Add adapter to `Platform` struct in `src/platform.rs`
- [ ] Add accessor method to Platform returning `&dyn Trait`
- [ ] Write tests for all three levels (Platform, WASM adapter, Native adapter)
- [ ] Test both targets: `cargo test --lib` and `wasm-pack test --headless --chrome`

### Migration Transform
```rust
// BEFORE
use crate::platform_module::function;
function("arg");

// AFTER
use crate::Platform;
let platform = Platform::new();
platform.port_name().method("arg");
```

## Pattern: Platform (Dependency Injection Container)

**Purpose:** Central container for all adapters, creates instances on demand.

```rust
// @location: src/platform.rs
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

    #[inline]
    pub fn clock(&self) -> &dyn ClockPort {
        &self.clock
    }

    #[inline]
    pub fn logger(&self) -> &dyn LoggerPort {
        &self.logger
    }

    #[inline]
    pub fn persistence(&self) -> &dyn PersistencePort {
        &self.persistence
    }
}
```

**Key points:**
- Stores concrete adapter instances (not references)
- Returns `&dyn Trait` from accessors (enables trait methods without import)
- `Copy` enables easy use in closures
- Zero-cost: all adapters are ZSTs

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
// Direct usage (stateless functions)
use crate::Platform;
let platform = Platform::new();
platform.port().operation("msg");

// In struct (stateful components)
pub struct Foo {
    platform: Platform,
}
impl Foo {
    pub fn new() -> Self {
        Self { platform: Platform::new() }
    }
    pub fn method(&self) {
        self.platform.port().operation("msg");
    }
}
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

### ❌ Forgetting Clone + Copy on Adapters
```rust
// WRONG - Platform won't be Copy
pub struct MyAdapter;

// CORRECT - enables Platform to be Copy
#[derive(Clone, Copy)]
pub struct MyAdapter;
```

### ❌ Returning Concrete Type from Platform
```rust
// WRONG - exposes implementation details
pub fn clock(&self) -> Clock {
    self.clock
}

// CORRECT - returns trait object
pub fn clock(&self) -> &dyn ClockPort {
    &self.clock
}
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

### ❌ Creating Platform in Loops
```rust
// WRONG - wasteful (even if zero-cost)
for item in items {
    let platform = Platform::new();
    platform.logger().log("...");
}

// CORRECT - create once
let platform = Platform::new();
for item in items {
    platform.logger().log("...");
}
```

## Native Implementation Pattern

### Simple Stub Example
```rust
// src/adapters/native/[name].rs
use crate::ports::PortName;

#[derive(Clone, Copy)]
pub struct Adapter;

impl Adapter {
    pub fn new() -> Self { Self }
}

impl PortName for Adapter {
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
# Shared dependencies
async-trait = "0.1"  # If using async trait methods

[dependencies.web-sys]
version = "0.3"
features = ["Window", "Performance", ...]  # WASM-specific APIs

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1", features = ["full"] }  # Native async runtime
```

**Key points:**
- No lazy_static needed (direct instantiation)
- WASM deps in main `[dependencies]` (only compiled for WASM target)
- Native-only deps in target-specific section

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

### When creating adapter:
`#[derive(Clone, Copy)] + pub fn new() -> Self`

### When using ports:
```rust
let platform = Platform::new();
platform.port().method();
```

### When adding to Platform:
1. Add adapter field to Platform struct
2. Initialize in `Platform::new()`
3. Add accessor returning `&dyn Trait`

### Compilation check:
```bash
cargo check --lib                    # Native
cargo test --lib                     # Native tests
wasm-pack test --headless --chrome   # WASM tests
```
