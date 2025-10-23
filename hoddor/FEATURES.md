# Hoddor Features

This document describes the cargo features available in the Hoddor crate.

## Available Features

### `default`
Default features include everything: `vault` + `graph`
```toml
hoddor = "1.3.0"  # includes vault + graph
```

### `minimal`
Just crypto primitives and storage abstraction.
- Age encryption (X25519)
- Argon2 key derivation
- OPFS storage (WASM) / FS storage (native)
- No vault operations, no graph database

```toml
hoddor = { version = "1.3.0", default-features = false, features = ["minimal"] }
```

**Use case**: Building custom storage solutions with just crypto + storage primitives.

### `vault`
Adds vault operations on top of `minimal`:
- Namespace management
- WebAuthn integration
- Vault encryption/decryption
- Identity management (passphrase + MFA)

```toml
hoddor = { version = "1.3.0", default-features = false, features = ["vault"] }
```

**Use case**: Encrypted vaults without graph database (e.g., simple key-value encrypted storage).

### `graph`
Adds CozoDB graph database with vector search.
- Requires `vault` for persistence and encryption
- Includes CozoDB in-memory graph (WASM) or compact storage (native)
- Vector similarity search (cosine similarity)
- Graph persistence with Age encryption

```toml
hoddor = { version = "1.3.0", default-features = false, features = ["graph"] }
```

**Use case**: Full-featured RAG, PKM, or any graph-based application.

## Feature Dependencies

```
graph
  └─ vault
       └─ minimal
            └─ console_error_panic_hook
```

## Size Comparison (WASM)

Approximate sizes for release builds:

- `minimal`: ~300KB (just crypto + storage)
- `vault`: ~400KB (+ vault operations)
- `graph`: ~900KB (+ CozoDB ~500KB)

## Examples

### Build with minimal features
```bash
wasm-pack build --target web --no-default-features --features minimal
```

### Build with vault only
```bash
wasm-pack build --target web --no-default-features --features vault
```

### Build with everything (default)
```bash
wasm-pack build --target web
```

## WASM Bindings

Each feature exposes different WASM functions:

**minimal**: Core crypto functions
- `generate_identity()`
- Age encryption/decryption primitives

**vault**: + Vault operations
- `create_vault()`
- `vault_identity_from_passphrase()`
- `create_credential()` / `get_credential()` (WebAuthn)

**graph**: + Graph operations
- `graph_create_memory_node()`
- `graph_vector_search()`
- `graph_list_memory_nodes()`
- `graph_backup_vault()` / `graph_restore_vault()`
