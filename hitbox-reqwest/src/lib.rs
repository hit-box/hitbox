//! Hitbox cache integration for reqwest HTTP client via reqwest-middleware.
//!
//! This crate provides [`CacheMiddleware`] for [`reqwest_middleware`] that adds
//! caching capabilities to the [`reqwest`] HTTP client using the hitbox caching
//! framework. Use [`CacheMiddleware::builder()`] to construct the middleware
//! with a fluent API.
//!
//! # Overview
//!
//! `hitbox-reqwest` enables **client-side HTTP caching** with:
//!
//! - **[Request and response predicates]** - Control what gets cached
//! - **[Cache key extraction]** - Build cache keys from request components
//! - **[Multiple backend support]** - Use [in-memory], [file storage], or [distributed] backends
//! - **[Dogpile prevention]** - Optional concurrency control to prevent thundering herd
//! - **Cache status headers** - Automatic `X-Cache-Status` header (HIT/MISS/STALE)
//!
//! [Request and response predicates]: hitbox_http::predicates
//! [Cache key extraction]: hitbox_http::extractors
//! [Multiple backend support]: hitbox_backend::composition
//! [in-memory]: hitbox_moka
//! [file storage]: hitbox_feoxdb
//! [distributed]: hitbox_redis
//! [Dogpile prevention]: BroadcastConcurrencyManager
//!
//! # Core Concepts
//!
//! - **[Predicate]**: A rule that determines if a request or response is cacheable.
//!   Predicates return [`Cacheable`] or [`NonCacheable`]. See [`hitbox_http::predicates`]
//!   for built-in predicates.
//! - **[Extractor]**: Generates cache key parts from HTTP components (method, path, headers).
//!   See [`hitbox_http::extractors`] for built-in extractors.
//! - **[Backend]**: Storage layer for cached responses. Available backends include
//!   [in-memory], [file storage], and [distributed] options.
//! - **[Policy]**: Controls TTL, stale-while-revalidate, and other caching behavior.
//! - **Dogpile effect**: When a cache entry expires, multiple concurrent requests may
//!   all attempt to refresh it simultaneously. Use [`BroadcastConcurrencyManager`] to prevent this.
//!
//! [Predicate]: hitbox_core::Predicate
//! [Extractor]: hitbox_core::Extractor
//! [Backend]: hitbox_backend::CacheBackend
//! [Policy]: hitbox::policy::PolicyConfig
//! [`Cacheable`]: hitbox_core::PredicateResult::Cacheable
//! [`NonCacheable`]: hitbox_core::PredicateResult::NonCacheable
//!
//! # Quick Start
//!
//! ## Basic Usage with Builder Pattern
//!
//! ```no_run
//! use std::time::Duration;
//! use reqwest::Client;
//! use reqwest_middleware::ClientBuilder;
//! use hitbox_reqwest::CacheMiddleware;
//! use hitbox_configuration::Endpoint;
//! use hitbox_http::{
//!     extractors::{Method as MethodExtractor, path::PathExtractor},
//!     predicates::request::Method,
//! };
//! use hitbox::policy::PolicyConfig;
//! use hitbox_moka::MokaBackend;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Create a cache backend (in-memory with 1000 entry capacity)
//! let backend = MokaBackend::builder(1000).build();
//!
//! // 2. Configure caching behavior
//! let config = Endpoint::builder()
//!     .request_predicate(Method::new(http::Method::GET).unwrap())  // Only cache GET
//!     .extractor(MethodExtractor::new().path("/{path}*"))          // Key from method+path
//!     .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
//!     .build();
//!
//! // 3. Create the middleware
//! let middleware = CacheMiddleware::builder()
//!     .backend(backend)
//!     .config(config)
//!     .build();
//!
//! // 4. Build the client
//! let client = ClientBuilder::new(Client::new())
//!     .with(middleware)
//!     .build();
//!
//! // 5. Make requests - caching happens automatically
//! let response = client.get("https://api.example.com/users").send().await?;
//!
//! // Check cache status via header
//! let cache_status = response.headers().get("X-Cache-Status");
//! // Returns "MISS" on first request, "HIT" on subsequent requests
//! # Ok(())
//! # }
//! ```
//!
//! # Response Headers
//!
//! The middleware adds an `X-Cache-Status` header to every response:
//!
//! | Value   | Meaning |
//! |---------|---------|
//! | `HIT`   | Response served from cache |
//! | `MISS`  | Response fetched from upstream (may be cached) |
//! | `STALE` | Stale cache served (background refresh may occur) |
//!
//! # Re-exports
//!
//! This crate re-exports commonly used types for convenience:
//!
//! - From [`hitbox_http`]: [`CacheableHttpRequest`], [`CacheableHttpResponse`],
//!   [`HttpEndpoint`], [`predicates`], [`extractors`]
//! - From [`hitbox`]: [`CacheConfig`], [`PolicyConfig`], concurrency managers
//! - From [`hitbox_core`]: [`DisabledOffload`]
//!
//! # Caveats
//!
//! - **No background revalidation**: Unlike `hitbox-tower`, this middleware uses
//!   [`DisabledOffload`] because `reqwest_middleware::Next<'_>` has a non-`'static`
//!   lifetime, preventing spawning of background tasks.
//!
//! # Internals
//!
//! On cache miss, the middleware uses [`ReqwestUpstream`] to call the next
//! middleware in the chain and convert between hitbox and reqwest types.
//!
//! [`reqwest`]: https://docs.rs/reqwest
//! [`reqwest_middleware`]: https://docs.rs/reqwest-middleware

mod middleware;
mod upstream;

pub use middleware::{CacheMiddleware, CacheMiddlewareBuilder};
pub use upstream::ReqwestUpstream;

// Re-export hitbox-http types for convenience
pub use hitbox_http::{
    BufferedBody, CacheableHttpRequest, CacheableHttpResponse, HttpEndpoint,
    SerializableHttpResponse, extractors, predicates,
};

/// Re-export reqwest body type for convenience in type annotations
pub use reqwest::Body as ReqwestBody;

// Re-export common types
pub use hitbox::config::CacheConfig;
pub use hitbox::policy::PolicyConfig;
pub use hitbox_core::DisabledOffload;

// Re-export concurrency types
pub use hitbox::concurrency::{
    BroadcastConcurrencyManager, ConcurrencyManager, NoopConcurrencyManager,
};
