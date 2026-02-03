# hitbox-http

HTTP caching primitives for the Hitbox framework.

This crate provides the building blocks for caching HTTP requests and responses:
predicates to determine cacheability, extractors to generate cache keys, and
body utilities for transparent request/response handling.

## Core Concepts

- **Predicate**: Evaluates whether a request or response should be cached.
  Returns [`Cacheable`] or [`NonCacheable`].

- **Extractor**: Generates cache key parts from HTTP components (method, path,
  headers, query parameters, body).

- **[`CacheableSubject`]**: A trait that allows predicates and extractors to work
  uniformly with both requests and responses.

- **[`BufferedBody`]**: A body wrapper with three states (`Complete`, `Partial`,
  `Passthrough`) enabling transparent caching without disrupting the HTTP stream.

## Quickstart

```rust,ignore
use std::time::Duration;

use hitbox::Config;
use hitbox::policy::PolicyConfig;
use hitbox::predicate::PredicateExt;
use hitbox_http::{
    extractors::{self, MethodConfig, MethodExtractor, PathExtractor, query::QueryExtractor},
    predicates::{
        header::Operation as HeaderOperation,
        request::{self, HeaderPredicate},
        response::{self, StatusCodePredicate, status},
   },
};

# use bytes::Bytes;
# use http_body_util::Empty;
// Build a cache configuration for an endpoint
let config = Config::builder()
    // Skip cache when Cache-Control: no-cache is present
    .request_predicate(
        request::predicate::<Empty<Bytes>>()
            .header(HeaderOperation::contains(http::header::CACHE_CONTROL, "no-cache"))
            .not(),
    )
    // Only cache successful responses
    .response_predicate(response::predicate::<Empty<Bytes>>().status(http::StatusCode::OK))
    // Build cache key from method, path parameters, and query
    .extractor(
        extractors::extractor::<Empty<Bytes>>()
            .method(MethodConfig::new())
            .path("/users/{user_id}/posts/{post_id}")
            .query("page"),
    )
    // Cache for 5 minutes
    .policy(PolicyConfig::builder().ttl(Duration::from_secs(300)).build())
    .build();
```

## Predicates

Predicates determine whether a request or response is cacheable.

### Request Predicates

| Predicate | Description |
|-----------|-------------|
| [`predicates::request::Method`] | Match by HTTP method (GET, POST, etc.) |
| [`predicates::request::Path`] | Match by path pattern |
| [`predicates::request::Header`] | Match by header presence or value |
| [`predicates::request::Query`] | Match by query parameter |
| [`predicates::request::Body`] | Match by request body content |

### Response Predicates

| Predicate | Description |
|-----------|-------------|
| [`predicates::response::StatusCode`] | Match by status code or class |
| [`predicates::response::Header`] | Match by header presence or value |
| [`predicates::response::Body`] | Match by response body content |

### Combining Predicates

Use [`PredicateExt`] methods to combine predicates:

```rust
use hitbox::predicate::PredicateExt;
use hitbox_http::predicates::header::{Header, Operation};

# use bytes::Bytes;
# use http_body_util::Empty;
# use hitbox::Neutral;
# use hitbox_http::CacheableHttpRequest;
# type Subject = CacheableHttpRequest<Empty<Bytes>>;

// Skip cache when Cache-Control contains "no-cache"
let skip_no_cache = Header::new(Operation::Contains(
    http::header::CACHE_CONTROL,
    "no-cache".to_string(),
));
# let _: &Header<Neutral<Subject>> = &skip_no_cache;
let skip_no_cache = skip_no_cache.not();

// Skip cache when Authorization header exists
let skip_auth = Header::new(Operation::Exist(
    http::header::AUTHORIZATION,
)).not();

// Combine: cache only if BOTH conditions pass
let combined = skip_no_cache.and(skip_auth);
```

## Extractors

Extractors generate cache key parts from HTTP components. Chain them using
the builder pattern:

```rust
use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathExtractor};
use hitbox_http::extractors::query::QueryExtractor;

# use bytes::Bytes;
# use http_body_util::Empty;
# use hitbox_http::extractors::{NeutralExtractor, Method, Path, query::Query};
let extractor = extractors::extractor::<Empty<Bytes>>()
    .method(MethodConfig::new())
    .path("/users/{user_id}")
    .query("page")
    .query("limit");
# let _: Query<Query<Path<Method<NeutralExtractor<Empty<Bytes>>>>>> = extractor;
```

| Extractor | Description |
|-----------|-------------|
| [`extractors::Method`] | Extract HTTP method |
| [`extractors::Path`] | Extract path parameters using patterns like `/users/{id}` |
| [`extractors::header`] | Extract header values |
| [`extractors::query`] | Extract query parameters |
| [`extractors::body`] | Extract from body (hash, JQ, regex) |
| [`extractors::Version`] | Extract HTTP version |

## Main Types

- [`CacheableHttpRequest`]: Wraps an HTTP request for cache evaluation.
- [`CacheableHttpResponse`]: Wraps an HTTP response for cache storage.
- [`SerializableHttpResponse`]: Serialized form of a response for cache backends.

## Feature Flags

- `rkyv_format`: Enables zero-copy deserialization using [rkyv](https://docs.rs/rkyv).

[`Cacheable`]: hitbox::predicate::PredicateResult::Cacheable
[`NonCacheable`]: hitbox::predicate::PredicateResult::NonCacheable
[`PredicateExt`]: hitbox::predicate::PredicateExt
