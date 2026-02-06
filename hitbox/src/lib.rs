#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Backend-related re-exports and utilities.
///
/// This module provides access to the [`Backend`](hitbox_backend::Backend) trait
/// and related types for implementing custom storage backends.
pub mod backend;

/// Dogpile prevention via concurrency management.
///
/// When a cache entry expires, multiple simultaneous requests can trigger redundant
/// upstream calls — the "thundering herd" problem. This module provides
/// [`BroadcastConcurrencyManager`](concurrency::BroadcastConcurrencyManager) to prevent this
/// by allowing only N requests to proceed while others wait for the result.
pub mod concurrency;

/// Error types for cache operations.
///
/// Defines [`CacheError`] which covers:
/// - Backend errors (storage failures)
/// - Upstream errors (data source failures)
/// - Cache key generation failures
pub mod error;

/// Finite State Machine for cache orchestration.
///
/// The FSM coordinates cache lookups, upstream calls, and response handling
/// based on cache state (hit, miss, stale) and configured policies.
pub mod fsm;

/// Metrics collection for cache observability.
///
/// When the `metrics` feature is enabled, this module provides counters
/// and histograms for:
/// - Cache hits, misses, and stale responses
/// - Request latency and upstream call timing
/// - Backend read/write operations
pub mod metrics;

pub use error::CacheError;

pub use hitbox_core::{
    And, BackendLabel, CacheKey, CachePolicy, CacheState, CacheValue, CacheablePolicyData,
    CacheableRequest, CacheableResponse, EntityPolicyConfig, Extractor, KeyPart, KeyParts, Neutral,
    Not, Or, Predicate, PredicateExt, Raw, RequestCachePolicy, ResponseCachePolicy,
};

/// Cache configuration types.
///
/// Provides types for configuring cache behavior including TTL, stale windows,
/// and endpoint-specific settings.
pub mod config;

/// Cache context and status types.
///
/// This module provides:
/// - [`CacheContext`](context::CacheContext) — metadata passed through the request lifecycle
/// - [`CacheStatus`](context::CacheStatus) — indicates whether a response came from cache
/// - [`ResponseSource`](context::ResponseSource) — identifies where the response originated
pub mod context;

/// Background task offloading for stale-while-revalidate.
///
/// When using the `OffloadRevalidate` stale policy, expired cache entries are served
/// immediately while fresh data is fetched in the background. This module provides
/// the [`OffloadManager`](offload::OffloadManager) for handling these background tasks.
pub mod offload;

pub use config::{
    CacheConfig, CacheConfigRouter, Config, ConfigBuilder, MultiConfig, NotSet, RouteMatch,
};
pub use context::{BoxContext, CacheContext, CacheStatus, CacheStatusExt, Context, ResponseSource};

/// Policy configuration for cache behavior.
///
/// Defines [`PolicyConfig`](policy::PolicyConfig) with:
/// - **TTL** — how long cached data remains fresh
/// - **Stale window** — grace period after TTL where stale data can be served
/// - **Stale policy** — how to handle stale data (`Return`, `Revalidate`, `OffloadRevalidate`)
/// - **Concurrency** — limit for dogpile prevention
pub mod policy;

/// Predicate trait and combinators for cache decisions.
///
/// Re-exports from [`hitbox-core`](https://docs.rs/hitbox-core) including:
/// - [`Predicate`] trait — determines if a request/response should be cached
/// - [`And`], [`Or`], [`Not`] — logical combinators for composing predicates
/// - [`Neutral`] — a predicate that always returns `Cacheable`
pub mod predicate {
    pub use hitbox_core::predicate::{
        And, Neutral, Not, Or, Predicate, PredicateExt, PredicateResult, combinators, neutral,
    };
}

/// Extractor trait for cache key generation.
///
/// Re-exports the [`Extractor`] trait from [`hitbox-core`](https://docs.rs/hitbox-core).
/// Extractors pull components from requests to build unique cache keys.
pub mod extractor {
    pub use hitbox_core::Extractor;
}

/// The `hitbox` prelude.
///
/// Provides convenient access to the most commonly used types:
///
/// ```rust
/// use hitbox::prelude::*;
/// ```
///
/// This imports:
/// - [`CacheError`] — error type for cache operations
/// - [`CacheableRequest`] — trait for cacheable request types
/// - [`CacheableResponse`] — trait for cacheable response types
pub mod prelude {
    pub use crate::{CacheError, CacheableRequest, CacheableResponse};
}
