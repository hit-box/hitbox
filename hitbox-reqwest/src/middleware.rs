//! Cache middleware for reqwest-middleware.
//!
//! This module provides [`CacheMiddleware`] which implements the
//! [`reqwest_middleware::Middleware`] trait to add caching capabilities
//! to reqwest HTTP clients.
//!
//! See the [crate-level documentation](crate) for usage examples.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use hitbox::CacheStatusExt;
use hitbox::backend::CacheBackend;
use hitbox::concurrency::{ConcurrencyManager, NoopConcurrencyManager};
use hitbox::config::CacheConfig;
use hitbox::fsm::CacheFuture;
use hitbox_core::DisabledOffload;
use hitbox_http::{
    BufferedBody, CacheableHttpRequest, CacheableHttpResponse, DEFAULT_CACHE_STATUS_HEADER,
};
use http::Extensions;
use http::header::HeaderName;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};

use crate::upstream::{ReqwestUpstream, buffered_body_to_reqwest};

/// Marker type for unset builder fields.
pub struct NotSet;

/// Cache middleware for reqwest-middleware.
///
/// This middleware intercepts HTTP requests and responses, caching them
/// according to the configured policy and predicates.
///
/// Use [`CacheMiddleware::builder()`] to construct an instance.
/// See the [crate-level documentation](crate) for usage examples.
pub struct CacheMiddleware<B, C, CM> {
    backend: Arc<B>,
    configuration: C,
    concurrency_manager: CM,
    /// Header name for cache status (HIT/MISS/STALE).
    cache_status_header: HeaderName,
}

impl<B, C, CM> CacheMiddleware<B, C, CM> {
    /// Creates a new cache middleware with explicit components.
    ///
    /// For most use cases, prefer [`CacheMiddleware::builder()`] which provides
    /// a more ergonomic API with sensible defaults.
    pub fn new(
        backend: Arc<B>,
        configuration: C,
        concurrency_manager: CM,
        cache_status_header: HeaderName,
    ) -> Self {
        Self {
            backend,
            configuration,
            concurrency_manager,
            cache_status_header,
        }
    }
}

impl CacheMiddleware<NotSet, NotSet, NoopConcurrencyManager> {
    /// Creates a new builder for constructing cache middleware.
    ///
    /// Both [`backend()`](CacheMiddlewareBuilder::backend) and
    /// [`config()`](CacheMiddlewareBuilder::config) must be called
    /// before [`build()`](CacheMiddlewareBuilder::build).
    ///
    /// See the [crate-level documentation](crate) for usage examples.
    pub fn builder() -> CacheMiddlewareBuilder<NotSet, NotSet, NoopConcurrencyManager> {
        CacheMiddlewareBuilder::new()
    }
}

impl<B, C, CM> Clone for CacheMiddleware<B, C, CM>
where
    C: Clone,
    CM: Clone,
{
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            configuration: self.configuration.clone(),
            concurrency_manager: self.concurrency_manager.clone(),
            cache_status_header: self.cache_status_header.clone(),
        }
    }
}

use async_trait::async_trait;

#[async_trait]
impl<B, C, CM> Middleware for CacheMiddleware<B, C, CM>
where
    B: CacheBackend + Send + Sync + 'static,
    C: CacheConfig<CacheableHttpRequest<reqwest::Body>, CacheableHttpResponse<reqwest::Body>>
        + Clone
        + Send
        + Sync
        + 'static,
    C::RequestPredicate: Clone + Send + Sync + 'static,
    C::ResponsePredicate: Clone + Send + Sync + 'static,
    C::Extractor: Clone + Send + Sync + 'static,
    CM: ConcurrencyManager<Result<CacheableHttpResponse<reqwest::Body>>>
        + Clone
        + Send
        + Sync
        + 'static,
{
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        // Convert reqwest::Request to http::Request<reqwest::Body>
        let http_request: http::Request<reqwest::Body> = req
            .try_into()
            .map_err(|e: reqwest::Error| reqwest_middleware::Error::Reqwest(e))?;

        // Wrap body with BufferedBody and create CacheableHttpRequest
        let (parts, body) = http_request.into_parts();
        let buffered_request =
            http::Request::from_parts(parts, BufferedBody::Passthrough(body));
        let cacheable_req = CacheableHttpRequest::from_request(buffered_request);

        // Create upstream wrapper
        let upstream = ReqwestUpstream::new(next.clone(), extensions.clone());

        // Create the cache future and box it to erase the problematic types
        // before capturing in the async_trait's boxed future
        let cache_future: Pin<Box<dyn Future<Output = _> + Send + '_>> = Box::pin(CacheFuture::new(
            self.backend.clone(),
            cacheable_req,
            upstream,
            self.configuration.request_predicates(),
            self.configuration.response_predicates(),
            self.configuration.extractors(),
            Arc::new(self.configuration.policy().clone()),
            DisabledOffload,
            self.concurrency_manager.clone(),
        ));

        // Execute cache future
        let (response, cache_context) = cache_future.await;

        // Convert CacheableHttpResponse back to reqwest::Response
        let mut cacheable_response = response?;

        // Add cache status header based on cache context
        cacheable_response.cache_status(cache_context.status, &self.cache_status_header);

        let http_response = cacheable_response.into_response();
        let (parts, buffered_body) = http_response.into_parts();

        // Convert BufferedBody back to reqwest::Body
        let body = buffered_body_to_reqwest(buffered_body);
        let http_response = http::Response::from_parts(parts, body);

        // Convert to reqwest::Response
        Ok(http_response.into())
    }
}

/// Builder for constructing [`CacheMiddleware`] with a fluent API.
///
/// Obtained via [`CacheMiddleware::builder()`].
/// Both [`backend()`](Self::backend) and [`config()`](Self::config)
/// must be called before [`build()`](Self::build).
///
/// See the [crate-level documentation](crate) for usage examples.
pub struct CacheMiddlewareBuilder<B, C, CM> {
    backend: B,
    configuration: C,
    concurrency_manager: CM,
    cache_status_header: Option<HeaderName>,
}

impl<B, C, CM> CacheMiddlewareBuilder<B, C, CM> {
    /// Sets the cache backend.
    pub fn backend<NB>(self, backend: NB) -> CacheMiddlewareBuilder<Arc<NB>, C, CM>
    where
        NB: CacheBackend,
    {
        CacheMiddlewareBuilder {
            backend: Arc::new(backend),
            configuration: self.configuration,
            concurrency_manager: self.concurrency_manager,
            cache_status_header: self.cache_status_header,
        }
    }

    /// Sets the cache configuration.
    ///
    /// Use [`Config::builder()`](hitbox::Config::builder) to create a configuration.
    pub fn config<NC>(self, configuration: NC) -> CacheMiddlewareBuilder<B, NC, CM> {
        CacheMiddlewareBuilder {
            backend: self.backend,
            configuration,
            concurrency_manager: self.concurrency_manager,
            cache_status_header: self.cache_status_header,
        }
    }

    /// Sets the concurrency manager for dogpile prevention.
    ///
    /// Defaults to [`NoopConcurrencyManager`](hitbox::concurrency::NoopConcurrencyManager) if not called.
    pub fn concurrency_manager<NCM>(
        self,
        concurrency_manager: NCM,
    ) -> CacheMiddlewareBuilder<B, C, NCM> {
        CacheMiddlewareBuilder {
            backend: self.backend,
            configuration: self.configuration,
            concurrency_manager,
            cache_status_header: self.cache_status_header,
        }
    }

    /// Sets the header name for cache status.
    ///
    /// The cache status header indicates whether a response was served from cache.
    /// Possible values are `HIT`, `MISS`, or `STALE`.
    ///
    /// Defaults to `x-cache-status` if not set.
    pub fn cache_status_header(self, header_name: HeaderName) -> Self {
        CacheMiddlewareBuilder {
            cache_status_header: Some(header_name),
            ..self
        }
    }
}

impl<B, C, CM> CacheMiddlewareBuilder<Arc<B>, C, CM>
where
    B: CacheBackend,
{
    /// Builds the cache middleware.
    ///
    /// Both [`backend()`](Self::backend) and [`config()`](Self::config) must
    /// be called before this method.
    pub fn build(self) -> CacheMiddleware<B, C, CM> {
        CacheMiddleware {
            backend: self.backend,
            configuration: self.configuration,
            concurrency_manager: self.concurrency_manager,
            cache_status_header: self
                .cache_status_header
                .unwrap_or(DEFAULT_CACHE_STATUS_HEADER),
        }
    }
}

impl CacheMiddlewareBuilder<NotSet, NotSet, NoopConcurrencyManager> {
    /// Creates a new builder. Equivalent to [`CacheMiddleware::builder()`].
    pub fn new() -> Self {
        Self {
            backend: NotSet,
            configuration: NotSet,
            concurrency_manager: NoopConcurrencyManager,
            cache_status_header: None,
        }
    }
}

impl Default for CacheMiddlewareBuilder<NotSet, NotSet, NoopConcurrencyManager> {
    fn default() -> Self {
        Self::new()
    }
}
