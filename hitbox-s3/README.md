# hitbox-s3

S3-compatible object storage backend for the Hitbox caching framework.

This crate provides [`S3Backend`], a cache backend that stores each entry as a
single object in Amazon S3 or any S3-compatible service (MinIO, Cloudflare R2,
Backblaze B2, Wasabi, S3 Express One Zone, ...). It is built on the
[`aws-sdk-s3`](https://docs.rs/aws-sdk-s3) client.

## When to Use This Backend

S3 trades latency for effectively unlimited, durable, shared capacity. Use it
as an **L3** tier behind faster caches — not on the hot path. A read is a full
network round-trip plus an object download.

| Backend | Typical read latency (5 KB) |
|---------|-----------------------------|
| Moka (in-memory) | ~2 µs |
| FeOxDB (embedded) | ~50 µs |
| Redis (localhost) | ~160 µs |
| **S3 (AWS Standard)** | **~50–200 ms** |

Reach for `hitbox-s3` when you need a large, durable, cross-instance cache that
survives restarts and is shared across many nodes, and you are composing it
behind a local L1/L2.

## Quickstart

```rust,no_run
use hitbox_s3::S3Backend;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let backend = S3Backend::builder("my-cache-bucket")
    .prefix("hitbox")
    .region("us-east-1")
    .build()
    .await?;
# Ok(())
# }
```

## S3-Compatible Services

For MinIO, R2, B2 and friends, set a custom endpoint and (optionally) static
credentials. Providing an endpoint automatically switches the client to
path-style addressing.

```rust,no_run
use hitbox_s3::S3Backend;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let backend = S3Backend::builder("my-cache-bucket")
    .endpoint("http://localhost:9000")
    .region("us-east-1")
    .credentials("minioadmin", "minioadmin")
    .build()
    .await?;
# Ok(())
# }
```

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `bucket` | *(required)* | Bucket that stores cache objects |
| `prefix` | none | Key prefix prepended to every object key |
| `endpoint` | AWS default | Custom endpoint URL (enables path-style addressing) |
| `region` | from environment | AWS region |
| `credentials` | default chain | Static access key / secret (MinIO/R2/B2) |
| `label` | `"s3"` | Backend label for metrics and composition |
| `value_format` | `BincodeFormat` | Value serialization format |
| `key_format` | `Bitcode` | Cache key serialization format |
| `compressor` | `PassthroughCompressor` | Value compression strategy |

Object keys are built as `{prefix}/{hex(serialized_key)}`. The serialized cache
key is hex-encoded so the result is always a valid, collision-free S3 key. Keys
that would exceed the S3 1024-byte limit are rejected with an error rather than
failing opaquely at the API.

## TTL and Expiration

S3 has **no native per-key TTL**. Each object body is a
[`ValueEnvelope`](hitbox_backend::ValueEnvelope): a small little-endian header
carrying the `expire`/`stale` timestamps (full sub-second precision), followed
by the raw value bytes (the payload is never re-serialized).

Expiration is enforced **lazily on read**: an entry past its `expire` timestamp
is returned as a cache miss. The object itself remains in the bucket until it is
physically removed.

To reclaim storage (and cost), configure an **S3 Lifecycle rule** to expire
objects under your cache prefix. For example, expire everything under
`hitbox/` after 1 day:

```json
{
  "Rules": [
    {
      "ID": "hitbox-cache-expiry",
      "Filter": { "Prefix": "hitbox/" },
      "Status": "Enabled",
      "Expiration": { "Days": 1 }
    }
  ]
}
```

This crate does not configure Lifecycle rules for you and runs no background
cleanup tasks; lazy read-time expiration plus operator-managed Lifecycle rules
are the cleanup model.

## Concurrency Model

Writes are **last-write-wins**. The backend is a plain storage adapter
(`GetObject` / `PutObject` / `DeleteObject`) with no compare-and-swap,
conditional writes, or version tokens. This matches the semantics of every
other Hitbox backend. If you need optimistic concurrency, layer it above the
cache.

## Multi-Tier Composition

`S3Backend` is intended to sit behind faster tiers. Compose it as the deepest
layer of an L1/L2/L3 hierarchy:

```rust,ignore
use hitbox_backend::composition::Compose;
use hitbox::offload::OffloadManager;
use hitbox_moka::MokaBackend;
use hitbox_s3::S3Backend;

let l1 = MokaBackend::builder().max_entries(10_000).build();
let l3 = S3Backend::builder("my-cache-bucket")
    .region("us-east-1")
    .build()
    .await?;

// Moka (L1) backed by S3 (L2/L3). Each `compose` takes an OffloadManager
// for background refill/revalidation.
let backend = l1.compose(l3, OffloadManager::with_defaults());
```

## Thread Safety

`S3Backend` is `Clone`, `Send`, and `Sync`. Cloned instances share the same
underlying AWS client (and its connection pool) via an internal `Arc`, so
cloning is cheap.

## Testing

Integration tests run against a MinIO container and are gated behind the
`integration` feature (they require Docker):

```bash
cargo test -p hitbox-s3 --features integration
```

Unit tests (key encoding, no I/O) run by default with `cargo test -p hitbox-s3`.

[`S3Backend`]: https://docs.rs/hitbox-s3/latest/hitbox_s3/struct.S3Backend.html
[`ValueEnvelope`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.ValueEnvelope.html
