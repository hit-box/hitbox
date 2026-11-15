# hitbox-backend

Backend abstraction layer for the Hitbox caching framework.

This crate provides the core traits and utilities for implementing cache backends.
It defines how cached data is stored, retrieved, serialized, and compressed.

## Overview

The crate is organized around several key concepts:

- **[`Backend`]** - Low-level dyn-compatible trait for raw byte storage operations (read/write/remove)
- **[`CacheBackend`]** - High-level trait with typed operations that handle serialization
- **[`Format`](format::Format)** - Serialization format abstraction (JSON, Bincode, Postcard, RON, Rkyv)
- **[`Compressor`]** - Compression abstraction (Passthrough, Gzip, Zstd)
- **[`CompositionBackend`]** - Multi-tier caching (L1/L2)

## Implementing a Backend

To implement your own backend, implement the [`Backend`] trait:

```rust
use std::collections::HashMap;
use std::sync::RwLock;
use hitbox_backend::{Backend, BackendResult, DeleteStatus};
use hitbox_core::{BackendLabel, CacheKey, CacheValue, Raw};
use async_trait::async_trait;

struct InMemoryBackend {
    store: RwLock<HashMap<CacheKey, CacheValue<Raw>>>,
}

#[async_trait]
impl Backend for InMemoryBackend {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        Ok(self.store.read().unwrap().get(key).cloned())
    }

    async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
        self.store.write().unwrap().insert(key.clone(), value);
        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        match self.store.write().unwrap().remove(key) {
            Some(_) => Ok(DeleteStatus::Deleted(1)),
            None => Ok(DeleteStatus::Missing),
        }
    }

    fn label(&self) -> BackendLabel {
        BackendLabel::new_static("in-memory")
    }

    // Optional: override defaults for value_format, key_format, compressor
}
```

Once you implement [`Backend`], you get [`CacheBackend`] for free via blanket implementation.
This provides typed `get`, `set`, and `delete` operations with automatic serialization.

## Feature Flags

- `gzip` - Enable Gzip compression via `GzipCompressor`
- `zstd` - Enable Zstd compression via `ZstdCompressor`
- `metrics` - Enable observability metrics for backend operations
- `rkyv_format` - Enable zero-copy Rkyv serialization via `RkyvFormat`

## Serialization Formats

| Format | Speed | Size | Human-readable |
|--------|-------|------|----------------|
| [`BincodeFormat`](format::BincodeFormat) | Fast | Compact | No |
| [`PostcardFormat`](format::PostcardFormat) | Fast | Compact | No** |
| [`JsonFormat`](format::JsonFormat) | Slow | Large | Partial* |
| [`RonFormat`](format::RonFormat) | Medium | Medium | Yes |
| `RkyvFormat` | Fastest | Compact | No |

*\* JSON serializes binary data as byte arrays `[104, 101, ...]`, not readable strings.*

*\*\* Postcard isn't self-describing: types that use `#[serde(untagged)]`, `#[serde(flatten)]`, `#[serde(skip_serializing_if)]` or `serde_json::Value` fail at runtime. Use JSON for them.*

## Compression Options

| Compressor | Ratio | Speed |
|------------|-------|-------|
| [`PassthroughCompressor`] | None | Fastest |
| `GzipCompressor` | Good | Medium |
| `ZstdCompressor` | Best | Fast |

## Multi-Tier Caching

Use [`CompositionBackend`] to combine backends into
L1/L2/L3 hierarchies:

```rust,ignore
use hitbox_backend::composition::Compose;

// Fast local cache (L1) with distributed cache fallback (L2)
let backend = moka.compose(redis, offload);
```

See the [`composition`] module for details on read/write/refill policies.
