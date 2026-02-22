# hitbox-tower

Tower middleware integration for the Hitbox caching framework.

This crate provides [`Cache`], a Tower [`Layer`] that adds transparent HTTP caching
to any Tower service. It evaluates requests against predicates, generates cache keys
using extractors, and stores/retrieves responses through pluggable backends.

## When to Use This Crate

Use `hitbox-tower` when you have a Tower-based HTTP service and want to add
caching. This works for both:

- **Server-side**: Wrap handlers in Axum, Hyper, or other Tower-based servers
- **Client-side**: Wrap HTTP clients like `hyper-util::Client` for response caching

## Core Concepts

- **[`Cache`]**: A Tower [`Layer`] that wraps services with caching behavior.
  Use [`Cache::builder()`] to configure and construct the layer.

- **[Predicate]**: Determines if a request/response should be cached.
  Returns `Cacheable` or `NonCacheable`. See [`hitbox_http::predicates`].

- **[Extractor]**: Generates cache key parts from HTTP components (method, path,
  headers, query etc.). See [`hitbox_http::extractors`].

- **[Backend]**: Storage for cached responses. Use [`hitbox_moka`] for in-memory,
  or other backends for distributed caching.

- **[Policy]**: Controls TTL, stale-while-revalidate, and other timing behavior.

[Predicate]: hitbox_core::Predicate
[Extractor]: hitbox_core::Extractor
[Backend]: hitbox::backend::CacheBackend
[Policy]: hitbox::policy::PolicyConfig
[`Layer`]: tower::Layer
[`hitbox_moka`]: https://docs.rs/hitbox-moka

## Quick Start

```rust
use std::time::Duration;
use hitbox::Config;
use hitbox::policy::PolicyConfig;
use hitbox_tower::Cache;
use hitbox_moka::MokaBackend;
use hitbox_http::extractors::{MethodConfig, MethodExtractor};
use hitbox_http::predicates::{NeutralRequestPredicate, NeutralResponsePredicate};
use hitbox_http::request;
use tower::{ServiceBuilder, service_fn};
# use http_body_util::Full;
# type Body = Full<bytes::Bytes>;

// 1. Create backend
let backend = MokaBackend::builder().max_entries(1000).build();

// 2. Configure caching behavior
let config = Config::builder()
    .request_predicate(NeutralRequestPredicate::new())
    .response_predicate(NeutralResponsePredicate::new())
    .extractor(request::extractor().method(MethodConfig::new()))
    .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
    .build();
# let _: Config<
#     NeutralRequestPredicate<Body>,
#     NeutralResponsePredicate<Body>,
#     hitbox_http::extractors::Method<hitbox_http::extractors::NeutralExtractor<Body>>,
# > = config;

// 3. Build the cache layer
let cache_layer = Cache::builder()
    .backend(backend)
    .config(config)
    .build();

// 4. Apply to a Tower service
let service = ServiceBuilder::new()
    .layer(cache_layer)
    .service(service_fn(|_req: http::Request<Body>| async {
        Ok::<_, std::convert::Infallible>(http::Response::new(Body::new("Hello".into())))
    }));
```

## Response Headers

The middleware adds a cache status header to every response:

| Header Value | Meaning |
|--------------|---------|
| `HIT` | Response served from cache |
| `MISS` | Response fetched from upstream (may be cached for future requests) |
| `STALE` | Stale cache entry served (background refresh may occur) |

The default header name is `x-cache-status`. Customize it with
[`CacheBuilder::cache_status_header`].

## Main Types

| Type | Description |
|------|-------------|
| [`Cache`] | Tower `Layer` — the main entry point |
| [`CacheBuilder`] | Fluent builder for configuring the cache layer |
| [`service::CacheService`] | The Tower `Service` that performs caching |
| [`TowerUpstream`] | Adapter bridging Tower services to Hitbox's upstream interface |

## Re-exports

This crate re-exports commonly used types for convenience:

- [`http::Method`], [`http::StatusCode`] — HTTP types for predicates
- [`CacheConfig`] — Trait for cache configuration
- [`Config`] — Generic cache configuration struct

For predicates and extractors, import from [`hitbox_http`]:

```rust
use hitbox_http::predicates::request::{self, MethodPredicate};
use hitbox_http::predicates::response::{self, StatusCodePredicate};
use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathExtractor};
```

## Examples

For complete, runnable examples see the `examples/` directory:

**Server-side caching:**
- **[tower.rs]** — Plain Tower service with Hyper server
- **[axum.rs]** — Axum web framework with per-route caching

**Client-side caching:**
- **[hyper_client.rs]** — Hyper client with response caching

Run with:
```text
cargo run -p hitbox-examples --example tower
cargo run -p hitbox-examples --example axum
cargo run -p hitbox-examples --example hyper_client
```

[tower.rs]: https://github.com/hit-box/hitbox/blob/main/examples/examples/tower.rs
[axum.rs]: https://github.com/hit-box/hitbox/blob/main/examples/examples/axum.rs
[hyper_client.rs]: https://github.com/hit-box/hitbox/blob/main/examples/examples/hyper_client.rs

## Feature Flags

This crate has no feature flags. Backend selection is done by depending on
the appropriate backend crate (e.g., `hitbox-moka`, `hitbox-redis`).
