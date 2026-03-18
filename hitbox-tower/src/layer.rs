//! Tower layer and builder for HTTP caching.
//!
//! This module provides [`Cache`], a Tower [`Layer`] that wraps services with
//! caching behavior, and [`CacheBuilder`] for fluent configuration.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use hitbox::Config;
//! use hitbox::policy::PolicyConfig;
//! use hitbox_tower::Cache;
//! use hitbox_moka::MokaBackend;
//! use hitbox_http::extractors::{MethodConfig, MethodExtractor};
//! use hitbox_http::request;
//!
//! # use http_body_util::Full;
//! # type Body = Full<bytes::Bytes>;
//! let config = Config::builder()
//!     .request_predicate(request::predicate::<Body>())
//!     .response_predicate(hitbox_http::response::predicate::<Body>())
//!     .extractor(request::extractor::<Body>().method(MethodConfig::new()))
//!     .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
//!     .build();
//!
//!
//! let cache_layer = Cache::builder()
//!     .backend(MokaBackend::builder().max_entries(1000).build())
//!     .config(config)
//!     .build();
//! ```
//!
//! [`Layer`]: tower::Layer

use std::sync::Arc;

use hitbox::backend::CacheBackend;
use hitbox::concurrency::NoopConcurrencyManager;
use hitbox_core::DisabledOffload;
use hitbox_http::HttpCacheStatusConfig;
use http::header::HeaderName;
use tower::Layer;

use crate::service::CacheService;

/// Marker type for unset builder fields.
pub struct NotSet;

/// Tower [`Layer`] that adds HTTP caching to a service.
///
/// `Cache` wraps any Tower service with caching behavior. When a request arrives,
/// the layer evaluates predicates to determine cacheability, generates a cache key
/// using extractors, and either returns a cached response or forwards to the
/// upstream service.
///
/// # Type Parameters
///
/// * `B` - Cache backend (e.g., [`MokaBackend`], `RedisBackend`). Must implement
///   [`CacheBackend`].
/// * `C` - Configuration providing predicates, extractors, and policy. Use
///   [`hitbox::Config`] to build custom configuration.
/// * `CM` - Concurrency manager for dogpile prevention. Use [`NoopConcurrencyManager`]
///   to disable or [`BroadcastConcurrencyManager`] to enable.
/// * `O` - Offload strategy for background revalidation. Use [`DisabledOffload`]
///   for synchronous behavior.
///
/// # Examples
///
/// Create with the builder pattern:
///
/// ```
/// use hitbox_tower::Cache;
/// use hitbox_moka::MokaBackend;
/// use hitbox::Config;
/// use hitbox_http::extractors::{MethodConfig, MethodExtractor};
/// use hitbox_http::request;
/// # use http_body_util::Full;
/// # type Body = Full<bytes::Bytes>;
///
/// let config = Config::builder()
///     .request_predicate(request::predicate::<Body>())
///     .response_predicate(hitbox_http::response::predicate::<Body>())
///     .extractor(request::extractor::<Body>().method(MethodConfig::new()))
///     .build();
///
///
/// let cache_layer = Cache::builder()
///     .backend(MokaBackend::builder().max_entries(1000).build())
///     .config(config)
///     .build();
/// ```
///
/// [`Layer`]: tower::Layer
/// [`MokaBackend`]: hitbox_moka::MokaBackend
/// [`CacheBackend`]: hitbox::backend::CacheBackend
/// [`NoopConcurrencyManager`]: hitbox::concurrency::NoopConcurrencyManager
/// [`BroadcastConcurrencyManager`]: hitbox::concurrency::BroadcastConcurrencyManager
/// [`DisabledOffload`]: hitbox_core::DisabledOffload
#[derive(Clone)]
pub struct Cache<B, C, CM, O = DisabledOffload> {
    /// The cache backend for storing and retrieving responses.
    pub backend: Arc<B>,
    /// Configuration with predicates, extractors, and cache policy.
    pub configuration: C,
    /// Offload strategy for background tasks.
    pub offload: O,
    /// Concurrency manager for dogpile prevention.
    pub concurrency_manager: CM,
    /// Configuration for cache status headers (RFC 9211 + legacy).
    pub cache_status_config: HttpCacheStatusConfig,
}

impl<S, B, C, CM, O> Layer<S> for Cache<B, C, CM, O>
where
    C: Clone,
    CM: Clone,
    O: Clone,
{
    type Service = CacheService<S, B, C, CM, O>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService::new(
            upstream,
            Arc::clone(&self.backend),
            self.configuration.clone(),
            self.offload.clone(),
            self.concurrency_manager.clone(),
            self.cache_status_config.clone(),
        )
    }
}

impl Cache<NotSet, NotSet, NoopConcurrencyManager, DisabledOffload> {
    /// Creates a new [`CacheBuilder`].
    ///
    /// Both [`backend()`](CacheBuilder::backend) and [`config()`](CacheBuilder::config)
    /// must be called before [`build()`](CacheBuilder::build).
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_tower::Cache;
    /// use hitbox_moka::MokaBackend;
    /// use hitbox::Config;
    /// use hitbox_http::extractors::{MethodConfig, MethodExtractor};
    /// use hitbox_http::request;
    /// # use http_body_util::Full;
    /// # type Body = Full<bytes::Bytes>;
    ///
    /// let config = Config::builder()
    ///     .request_predicate(request::predicate::<Body>())
    ///     .response_predicate(hitbox_http::response::predicate::<Body>())
    ///     .extractor(request::extractor::<Body>().method(MethodConfig::new()))
    ///     .build();
    ///
    ///
    /// let cache_layer = Cache::builder()
    ///     .backend(MokaBackend::builder().max_entries(1000).build())
    ///     .config(config)
    ///     .build();
    /// ```
    pub fn builder() -> CacheBuilder<NotSet, NotSet, NoopConcurrencyManager, DisabledOffload> {
        CacheBuilder::new()
    }
}

/// Fluent builder for constructing a [`Cache`] layer.
///
/// Use [`Cache::builder()`] to create a new builder. Both [`backend()`](Self::backend)
/// and [`config()`](Self::config) must be called before [`build()`](Self::build).
///
/// # Type Parameters
///
/// The type parameters change as you call builder methods:
///
/// * `B` - Backend type, set by [`backend()`](Self::backend)
/// * `C` - Configuration type, set by [`config()`](Self::config)
/// * `CM` - Concurrency manager type, set by [`concurrency_manager()`](Self::concurrency_manager)
/// * `O` - Offload type, set by [`offload()`](Self::offload)
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use hitbox_tower::Cache;
/// use hitbox_moka::MokaBackend;
/// use hitbox::Config;
/// use hitbox::policy::PolicyConfig;
/// use hitbox_http::extractors::{MethodConfig, MethodExtractor};
/// use hitbox_http::request;
/// use http::header::HeaderName;
/// # use http_body_util::Full;
/// # type Body = Full<bytes::Bytes>;
///
/// let config = Config::builder()
///     .request_predicate(request::predicate::<Body>())
///     .response_predicate(hitbox_http::response::predicate::<Body>())
///     .extractor(request::extractor::<Body>().method(MethodConfig::new()))
///     .policy(PolicyConfig::builder().ttl(Duration::from_secs(300)).build())
///     .build();
///
///
/// let layer = Cache::builder()
///     .backend(MokaBackend::builder().max_entries(10_000).build())
///     .config(config)
///     .cache_status_header(HeaderName::from_static("x-custom-cache"))
///     .build();
/// ```
pub struct CacheBuilder<B, C, CM, O = DisabledOffload> {
    backend: B,
    configuration: C,
    offload: O,
    concurrency_manager: CM,
    cache_status_config: Option<HttpCacheStatusConfig>,
}

impl CacheBuilder<NotSet, NotSet, NoopConcurrencyManager, DisabledOffload> {
    /// Creates a new builder.
    ///
    /// Prefer using [`Cache::builder()`] instead of calling this directly.
    pub fn new() -> Self {
        Self {
            backend: NotSet,
            configuration: NotSet,
            offload: DisabledOffload,
            concurrency_manager: NoopConcurrencyManager,
            cache_status_config: None,
        }
    }
}

impl Default for CacheBuilder<NotSet, NotSet, NoopConcurrencyManager, DisabledOffload> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B, C, CM, O> CacheBuilder<B, C, CM, O> {
    /// Sets the cache backend for storing responses.
    ///
    /// Common backends:
    ///
    /// - `MokaBackend` — In-memory cache (from `hitbox-moka`)
    /// - `RedisBackend` — Distributed cache via Redis (from `hitbox-redis`)
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_tower::Cache;
    /// use hitbox_moka::MokaBackend;
    ///
    /// let builder = Cache::builder()
    ///     .backend(MokaBackend::builder().max_entries(1000).build());
    /// ```
    pub fn backend<NB: CacheBackend>(self, backend: NB) -> CacheBuilder<NB, C, CM, O> {
        CacheBuilder {
            backend,
            configuration: self.configuration,
            offload: self.offload,
            concurrency_manager: self.concurrency_manager,
            cache_status_config: self.cache_status_config,
        }
    }

    /// Sets the cache configuration with predicates, extractors, and policy.
    ///
    /// Use [`Config::builder()`](hitbox::Config::builder) to create a configuration with:
    /// - Request predicates (which requests to cache)
    /// - Response predicates (which responses to cache)
    /// - Extractors (how to generate cache keys)
    /// - Policy (TTL, stale handling)
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use hitbox_tower::Cache;
    /// use hitbox_moka::MokaBackend;
    /// use hitbox::Config;
    /// use hitbox::policy::PolicyConfig;
    /// use hitbox_http::extractors::{MethodConfig, MethodExtractor};
    /// use hitbox_http::request;
    /// # use http_body_util::Full;
    /// # type Body = Full<bytes::Bytes>;
    ///
    /// let config = Config::builder()
    ///     .request_predicate(request::predicate::<Body>())
    ///     .response_predicate(hitbox_http::response::predicate::<Body>())
    ///     .extractor(request::extractor::<Body>().method(MethodConfig::new()))
    ///     .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
    ///     .build();
    ///
    ///
    /// let layer = Cache::builder()
    ///     .backend(MokaBackend::builder().max_entries(1000).build())
    ///     .config(config)
    ///     .build();
    /// ```
    pub fn config<NC>(self, configuration: NC) -> CacheBuilder<B, NC, CM, O> {
        CacheBuilder {
            backend: self.backend,
            configuration,
            offload: self.offload,
            concurrency_manager: self.concurrency_manager,
            cache_status_config: self.cache_status_config,
        }
    }

    /// Sets the concurrency manager for dogpile prevention.
    ///
    /// The dogpile effect occurs when a cache entry expires and multiple
    /// concurrent requests all try to refresh it simultaneously. A concurrency
    /// manager prevents this by coordinating requests.
    ///
    /// Options:
    /// - [`NoopConcurrencyManager`] — No coordination (default)
    /// - [`BroadcastConcurrencyManager`] — One request fetches, others wait
    ///
    /// [`NoopConcurrencyManager`]: hitbox::concurrency::NoopConcurrencyManager
    /// [`BroadcastConcurrencyManager`]: hitbox::concurrency::BroadcastConcurrencyManager
    pub fn concurrency_manager<NCM>(self, concurrency_manager: NCM) -> CacheBuilder<B, C, NCM, O> {
        CacheBuilder {
            backend: self.backend,
            configuration: self.configuration,
            offload: self.offload,
            concurrency_manager,
            cache_status_config: self.cache_status_config,
        }
    }

    /// Sets the offload strategy for background revalidation.
    ///
    /// When serving stale content, the offload strategy determines how
    /// background refresh is performed.
    ///
    /// Defaults to [`DisabledOffload`] (synchronous revalidation).
    ///
    /// [`DisabledOffload`]: hitbox_core::DisabledOffload
    pub fn offload<NO>(self, offload: NO) -> CacheBuilder<B, C, CM, NO> {
        CacheBuilder {
            backend: self.backend,
            configuration: self.configuration,
            offload,
            concurrency_manager: self.concurrency_manager,
            cache_status_config: self.cache_status_config,
        }
    }

    /// Sets the legacy header name for cache status.
    ///
    /// Controls the legacy `x-cache-status` header name. The RFC 9211
    /// `Cache-Status` header is always included.
    ///
    /// Defaults to [`hitbox_http::DEFAULT_CACHE_STATUS_HEADER`] (`x-cache-status`).
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_tower::Cache;
    /// use hitbox_moka::MokaBackend;
    /// use http::header::HeaderName;
    ///
    /// let builder = Cache::builder()
    ///     .backend(MokaBackend::builder().max_entries(1000).build())
    ///     .cache_status_header(HeaderName::from_static("x-custom-cache"));
    /// ```
    pub fn cache_status_header(self, header_name: HeaderName) -> Self {
        let mut config = self.cache_status_config.unwrap_or_default();
        config.legacy_header = Some(header_name);
        CacheBuilder {
            cache_status_config: Some(config),
            ..self
        }
    }

    /// Sets the cache name used in the RFC 9211 `Cache-Status` header.
    ///
    /// Defaults to `"hitbox"`.
    pub fn cache_name(self, name: impl Into<String>) -> Self {
        let mut config = self.cache_status_config.unwrap_or_default();
        config.cache_name = name.into();
        CacheBuilder {
            cache_status_config: Some(config),
            ..self
        }
    }
}

impl<B, C, CM, O> CacheBuilder<B, C, CM, O>
where
    B: CacheBackend,
{
    /// Builds the [`Cache`] layer.
    ///
    /// Both [`backend()`](Self::backend) and [`config()`](Self::config) must
    /// be called before this method.
    pub fn build(self) -> Cache<B, C, CM, O> {
        Cache {
            backend: Arc::new(self.backend),
            configuration: self.configuration,
            offload: self.offload,
            concurrency_manager: self.concurrency_manager,
            cache_status_config: self.cache_status_config.unwrap_or_default(),
        }
    }
}
