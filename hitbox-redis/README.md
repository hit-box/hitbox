# hitbox-redis

Redis cache backend for the [Hitbox] caching framework.

This crate provides [`RedisBackend`], a cache backend powered by
[redis-rs](https://github.com/redis-rs/redis-rs). It supports both single-node
Redis instances and Redis Cluster deployments (with the `cluster` feature).

## Overview

- **Single-node or cluster**: Connect to a single Redis instance or a Redis Cluster
- **Multiplexed connection**: Efficient connection reuse via [`ConnectionManager`]
- **Automatic TTL**: Entries expire using native Redis TTL mechanism
- **Lazy connection**: Connection established on first operation, not at construction

## Quickstart

### Single Node

```rust
use hitbox_redis::{RedisBackend, ConnectionMode};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let backend = RedisBackend::builder()
    .connection(ConnectionMode::single("redis://localhost:6379/"))
    .build()?;
# Ok(())
# }
```

### Cluster

Requires the `cluster` feature.

```rust
# #[cfg(feature = "cluster")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use hitbox_redis::{RedisBackend, ConnectionMode};

let backend = RedisBackend::builder()
    .connection(ConnectionMode::cluster([
        "redis://node1:6379",
        "redis://node2:6379",
        "redis://node3:6379",
    ]))
    .build()?;
# Ok(())
# }
# #[cfg(not(feature = "cluster"))]
# fn main() {}
```

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `connection` | (required) | Connection mode (single or cluster) |
| `username` | None | Redis 6+ ACL username |
| `password` | None | Redis password |
| `key_format` | [`Bitcode`] | Cache key serialization format |
| `value_format` | [`PostcardFormat`] | Value serialization format |
| `compressor` | [`PassthroughCompressor`] | Compression strategy |
| `label` | `"redis"` | Backend label for multi-tier composition |

### Serialization Formats

The `value_format` option controls how cached data is serialized. Available formats
are provided by [`hitbox_backend::format`]:

| Format | Speed | Size | Human-readable | Use case |
|--------|-------|------|----------------|----------|
| [`PostcardFormat`] | Fast | Compact | No** | General purpose (default) |
| [`JsonFormat`] | Slow | Large | Partial* | Debugging, interoperability |
| [`RonFormat`] | Medium | Medium | Yes | Config files, debugging |
| [`RkyvFormat`] | Fastest | Compact | No | Zero-copy, max performance |

*\* JSON serializes binary data as byte arrays `[104, 101, ...]`, not readable strings.*

*\*\* Postcard isn't self-describing: types that use `#[serde(untagged)]`, `#[serde(flatten)]`, `#[serde(skip_serializing_if)]` or `serde_json::Value` fail at runtime. Use JSON for them.*

**Changing the format.** Stored entries don't record which format or compressor wrote them. When you change `value_format` or the compressor, or change a cached type, use a new Redis database or flush the current one: otherwise old entries are read with the new settings, and while most fail to decode and get replaced, some can decode into wrong values.

**Note:** [`RkyvFormat`] requires enabling the `rkyv_format` feature on `hitbox-backend`.

### Key Formats

The `key_format` option controls how [`CacheKey`] values are serialized for Redis:

| Format | Size | Human-readable | Use case |
|--------|------|----------------|----------|
| [`Bitcode`] | Compact | No | Production (default, recommended) |
| [`UrlEncoded`] | Larger | Yes | Debugging, CDN/HTTP integration |

### Compression Strategies

The `compressor` option controls whether cached data is compressed. Available
compressors are provided by [`hitbox_backend`]:

| Compressor | Ratio | Speed | Feature flag |
|------------|-------|-------|--------------|
| [`PassthroughCompressor`] | None | Fastest | — |
| [`GzipCompressor`] | Good | Medium | `gzip` |
| [`ZstdCompressor`] | Best | Fast | `zstd` |

For Redis backends, compression is often **recommended** since it reduces:

- Network bandwidth between application and Redis
- Redis memory usage
- Storage costs for Redis persistence (RDB/AOF)

Consider compression when cached values exceed ~1KB.

## When to Use This Backend

Use `RedisBackend` when you need:

- **Distributed caching**: Share cache across multiple application instances
- **Persistence**: Survive application restarts (with Redis RDB/AOF)
- **Large capacity**: Store more data than fits in process memory

Consider other backends when you need:

- **Lowest latency**: Use [`hitbox-moka`] for in-process caching
- **Both**: Use multi-tier composition (Moka L1 + Redis L2)

## Multi-Tier Composition

`RedisBackend` works well as an L2 cache behind a fast in-memory L1:

```rust
use hitbox::offload::OffloadManager;
use hitbox_backend::composition::Compose;
use hitbox_moka::MokaBackend;
use hitbox_redis::{RedisBackend, ConnectionMode};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Fast local cache (L1) backed by Redis (L2)
let l1 = MokaBackend::builder().max_entries(10_000).build();
let l2 = RedisBackend::builder()
    .connection(ConnectionMode::single("redis://localhost:6379/"))
    .build()?;

let offload = OffloadManager::with_defaults();
let composed = l1.compose(l2, offload);
# Ok(())
# }
```

[Hitbox]: hitbox
[`Bitcode`]: hitbox_backend::CacheKeyFormat::Bitcode
[`UrlEncoded`]: hitbox_backend::CacheKeyFormat::UrlEncoded
[`PostcardFormat`]: hitbox_backend::format::PostcardFormat
[`JsonFormat`]: hitbox_backend::format::JsonFormat
[`RonFormat`]: hitbox_backend::format::RonFormat
[`RkyvFormat`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/format/struct.RkyvFormat.html
[`PassthroughCompressor`]: hitbox_backend::PassthroughCompressor
[`GzipCompressor`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.GzipCompressor.html
[`ZstdCompressor`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.ZstdCompressor.html
[`hitbox_backend::format`]: hitbox_backend::format
[`hitbox-moka`]: https://docs.rs/hitbox-moka
[`CacheKey`]: hitbox::CacheKey
[`ConnectionManager`]: redis::aio::ConnectionManager
