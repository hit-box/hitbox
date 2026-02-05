# hitbox-feoxdb

Persistent cache backend for the Hitbox caching framework.

This crate provides [`FeOxDbBackend`], a disk-based cache backend powered by
[FeOxDB](https://github.com/mehrantsi/feoxdb), a pure Rust embedded database.
It offers data persistence across restarts and native per-key TTL support.

## Overview

- **Persistence**: Cache data survives application restarts
- **Per-key TTL**: Each entry can have its own expiration time
- **Pure Rust**: No external dependencies or system libraries required
- **In-memory mode**: Optional ephemeral storage for testing

## Quickstart

```rust,no_run
use hitbox_feoxdb::FeOxDbBackend;

// Create a persistent backend
let backend = FeOxDbBackend::builder()
    .path("/var/cache/myapp")
    .build()
    .expect("Failed to open database");

// Or use in-memory mode for testing
let test_backend = FeOxDbBackend::in_memory()
    .expect("Failed to create in-memory backend");
```

## Storage Modes

FeOxDbBackend supports two storage modes:

| Mode | Method | Use case |
|------|--------|----------|
| Persistent | `builder().path(...)` | Production, data survives restarts |
| In-memory | `in_memory()` | Testing, temporary caching |

When using persistent mode with a directory path, the database file is
automatically created as `cache.db` within that directory.

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `path` | None (in-memory) | Database file path |
| `max_file_size` | 1 GB | Maximum disk storage capacity |
| `max_memory` | 1 GB | Maximum RAM usage |
| `key_format` | [`Bitcode`] | Cache key serialization format |
| `value_format` | [`JsonFormat`] | Value serialization format |
| `compressor` | [`PassthroughCompressor`] | Compression strategy |
| `label` | `"feoxdb"` | Backend label for multi-tier composition |

### Resource Limits

FeOxDB allows configuring resource limits for both disk and memory usage:

- **`max_file_size`**: Controls the maximum size of the database file on disk.
  The file is pre-allocated at this size. When full, writes fail with `OutOfSpace`.
  Only applies to persistent mode.

- **`max_memory`**: Controls RAM usage for in-memory data structures.
  - In **memory-only mode**: This is the total storage capacity
  - In **persistent mode**: This limits the in-memory cache for disk data

  When exceeded, writes fail with `OutOfMemory`. FeOxDB does not automatically
  evict entries from the main store (unlike Moka's LRU eviction).

```rust,no_run
use hitbox_feoxdb::FeOxDbBackend;

let backend = FeOxDbBackend::builder()
    .path("/var/cache/myapp")
    .max_file_size(10 * 1024 * 1024 * 1024)  // 10 GB disk limit
    .max_memory(512 * 1024 * 1024)           // 512 MB RAM limit
    .build()
    .expect("Failed to open database");
```

### Serialization Formats

The `value_format` option controls how cached data is serialized. Available formats
are provided by [`hitbox_backend::format`]:

| Format | Speed | Size | Human-readable | Use case |
|--------|-------|------|----------------|----------|
| [`JsonFormat`] | Slow | Large | Partial* | Debugging, interoperability |
| [`BincodeFormat`] | Fast | Compact | No | General purpose (recommended) |
| [`RonFormat`] | Medium | Medium | Yes | Config files, debugging |
| [`RkyvFormat`] | Fastest | Compact | No | Zero-copy, max performance |

*\* JSON serializes binary data as byte arrays `[104, 101, ...]`, not readable strings.*

**Note:** [`RkyvFormat`] requires enabling the `rkyv_format` feature on `hitbox-backend`.

### Compression Strategies

The `compressor` option controls whether cached data is compressed. Available
compressors are provided by [`hitbox_backend`]:

| Compressor | Ratio | Speed | Feature flag |
|------------|-------|-------|--------------|
| [`PassthroughCompressor`] | None | Fastest | — |
| [`GzipCompressor`] | Good | Medium | `gzip` |
| [`ZstdCompressor`] | Best | Fast | `zstd` |

For disk-based caches, compression is often **recommended** since it reduces
I/O operations and disk usage, which can improve performance.

## TTL Handling

FeOxDB natively supports per-key TTL. When writing a cache entry with an
expiration time, the TTL is computed and passed to FeOxDB's `insert_with_ttl`.
Expired entries are automatically cleaned up by the database.

As a safety measure, the backend also checks expiration during reads to
handle edge cases where cleanup hasn't occurred yet.

## When to Use This Backend

Use `FeOxDbBackend` when you need:

- **Persistence**: Cache data must survive application restarts
- **Single-instance**: Data doesn't need to be shared across processes
- **Simplicity**: No external services to manage (unlike Redis)
- **Large datasets**: Data that doesn't fit in memory

Consider other backends when you need:

- **Distributed caching**: Use [`hitbox-redis`] instead
- **Maximum speed**: Use [`hitbox-moka`] for pure in-memory caching
- **Memory-bounded cache**: Use [`hitbox-moka`] with LRU eviction

## Multi-Tier Composition

`FeOxDbBackend` works well as an L2 cache behind a fast in-memory cache:

```rust,ignore
use hitbox_backend::composition::Compose;
use hitbox_moka::MokaBackend;
use hitbox_feoxdb::FeOxDbBackend;

// Fast in-memory cache (L1) backed by disk (L2)
let l1 = MokaBackend::builder(10_000).build();
let l2 = FeOxDbBackend::builder()
    .path("/var/cache/myapp")
    .build()?;

let backend = l1.compose(l2, offload_manager);
```

## Thread Safety

`FeOxDbBackend` is `Clone`, `Send`, and `Sync`. Cloned instances share the
same underlying database connection via `Arc<FeoxStore>`. All database
operations are performed in blocking tasks to avoid blocking the async runtime.

[`Bitcode`]: hitbox_backend::CacheKeyFormat::Bitcode
[`JsonFormat`]: hitbox_backend::format::JsonFormat
[`BincodeFormat`]: hitbox_backend::format::BincodeFormat
[`RonFormat`]: hitbox_backend::format::RonFormat
[`RkyvFormat`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/format/struct.RkyvFormat.html
[`PassthroughCompressor`]: hitbox_backend::PassthroughCompressor
[`GzipCompressor`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.GzipCompressor.html
[`ZstdCompressor`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.ZstdCompressor.html
[`hitbox_backend::format`]: hitbox_backend::format
[`hitbox-redis`]: https://docs.rs/hitbox-redis
[`hitbox-moka`]: https://docs.rs/hitbox-moka
