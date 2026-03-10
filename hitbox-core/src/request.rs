//! Cacheable request types and traits.
//!
//! This module provides types for determining whether requests should be
//! cached and extracting cache keys from them:
//!
//! - [`CacheableRequest`] - Trait for request types that can participate in caching
//! - [`CacheablePolicyData`] - Request bundled with its cache key
//! - [`RequestCachePolicy`] - Type alias for request cache decisions
//!
//! ## Request Processing Flow
//!
//! When a request is processed:
//!
//! 1. **Predicates** evaluate whether the request should be cached
//! 2. **Extractors** generate the cache key from request components
//! 3. The result is either `Cacheable` (with key) or `NonCacheable`

use std::future::Future;

use crate::{CacheKey, CachePolicy, extractor::Extractor, predicate::Predicate};

/// A cacheable request bundled with its generated cache key.
///
/// Created when a request passes predicate evaluation and has its
/// cache key extracted. Contains both the original request and the
/// key used for cache lookup/storage.
pub struct CacheablePolicyData<T> {
    /// The generated cache key for this request.
    pub key: CacheKey,
    /// The original request.
    pub request: T,
}

impl<T> CacheablePolicyData<T> {
    /// Creates a new cacheable policy data with the given key and request.
    pub fn new(key: CacheKey, request: T) -> Self {
        CacheablePolicyData { key, request }
    }
}

/// Cache policy for requests.
///
/// Type alias that specializes [`CachePolicy`] for request caching:
/// - `Cacheable` variant contains [`CacheablePolicyData`] with the request and its key
/// - `NonCacheable` variant contains the original request
pub type RequestCachePolicy<T> = CachePolicy<CacheablePolicyData<T>, T>;

/// Trait for request types that can participate in caching.
///
/// Implementations determine whether a request should be cached by
/// applying predicates and extracting cache keys.
///
/// # Type Requirements
///
/// Request types must be `Sized` to allow ownership transfer through
/// the caching pipeline.
///
/// # Processing
///
/// The `cache_policy` method:
/// 1. Applies predicates to determine if the request is cacheable
/// 2. If cacheable, extracts a cache key using the provided extractors
/// 3. Returns either `Cacheable` with the key or `NonCacheable`
pub trait CacheableRequest: Sized {
    /// The future type returned by `cache_policy`.
    ///
    /// Uses GAT to properly capture the lifetime of `Self`, allowing
    /// request types with non-`'static` references.
    type CachePolicyFuture<'a, P, E>: Future<Output = RequestCachePolicy<Self>> + Send + 'a
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a;

    /// Determine if this request should be cached and extract its key.
    ///
    /// # Arguments
    ///
    /// * `predicates` - Predicates to evaluate whether the request is cacheable
    /// * `extractors` - Extractors to generate the cache key
    fn cache_policy<'a, P, E>(
        self,
        predicates: P,
        extractors: E,
    ) -> Self::CachePolicyFuture<'a, P, E>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a;
}
