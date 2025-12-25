//! Cache middleware for reqwest-middleware.
//!
//! This module provides [`CacheMiddleware`] which implements the
//! [`reqwest_middleware::Middleware`] trait to add caching capabilities
//! to reqwest HTTP clients.
//!
//! See the [crate-level documentation](crate) for usage examples.

use std::sync::Arc;

use async_trait::async_trait;
use hitbox::backend::CacheBackend;
use hitbox::concurrency::{ConcurrencyManager, NoopConcurrencyManager};
use hitbox::config::CacheConfig;
use hitbox::context::CacheStatus;
use hitbox::fsm::CacheFuture;
use hitbox_core::DisabledOffload;
use hitbox_http::{BufferedBody, CacheableHttpRequest, CacheableHttpResponse, HttpEndpoint};
use http::Extensions;
use http::header::HeaderValue;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};

use crate::upstream::{ReqwestUpstream, buffered_body_to_reqwest};

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
}

impl<B, C, CM> CacheMiddleware<B, C, CM> {
    /// Creates a new cache middleware with explicit components.
    ///
    /// For most use cases, prefer [`CacheMiddleware::builder()`] which provides
    /// a more ergonomic API with sensible defaults.
    pub fn new(backend: Arc<B>, configuration: C, concurrency_manager: CM) -> Self {
        Self {
            backend,
            configuration,
            concurrency_manager,
        }
    }
}

impl CacheMiddleware<(), HttpEndpoint, NoopConcurrencyManager> {
    /// Creates a new builder for constructing cache middleware.
    ///
    /// See the [crate-level documentation](crate) for usage examples.
    pub fn builder() -> CacheMiddlewareBuilder<(), HttpEndpoint, NoopConcurrencyManager> {
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
        }
    }
}

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
        let buffered_request = http::Request::from_parts(parts, BufferedBody::Passthrough(body));
        let cacheable_req = CacheableHttpRequest::from_request(buffered_request);

        // Create upstream wrapper
        let upstream = ReqwestUpstream::new(next.clone(), extensions.clone());

        // Create CacheFuture with DisabledOffload (no background revalidation)
        // This allows us to use non-'static lifetimes
        let cache_future: CacheFuture<
            '_,
            B,
            CacheableHttpRequest<reqwest::Body>,
            Result<CacheableHttpResponse<reqwest::Body>>,
            ReqwestUpstream<'_>,
            C::RequestPredicate,
            C::ResponsePredicate,
            C::Extractor,
            CM,
            DisabledOffload,
        > = CacheFuture::new(
            self.backend.clone(),
            cacheable_req,
            upstream,
            self.configuration.request_predicates(),
            self.configuration.response_predicates(),
            self.configuration.extractors(),
            Arc::new(self.configuration.policy().clone()),
            DisabledOffload,
            self.concurrency_manager.clone(),
        );

        // Execute cache future
        let (response, cache_context) = cache_future.await;

        // Convert CacheableHttpResponse back to reqwest::Response
        let cacheable_response = response?;
        let mut http_response = cacheable_response.into_response();

        // Add X-Cache-Status header based on cache context
        let status_value = match cache_context.status {
            CacheStatus::Hit => HeaderValue::from_static("HIT"),
            CacheStatus::Miss => HeaderValue::from_static("MISS"),
            CacheStatus::Stale => HeaderValue::from_static("STALE"),
        };
        http_response
            .headers_mut()
            .insert("X-Cache-Status", status_value);

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
/// See the [crate-level documentation](crate) for usage examples.
pub struct CacheMiddlewareBuilder<B, C, CM> {
    backend: Option<Arc<B>>,
    configuration: C,
    concurrency_manager: CM,
}

impl<B, C, CM> CacheMiddlewareBuilder<B, C, CM> {
    /// Sets the cache backend (**required**).
    ///
    /// # Panics
    ///
    /// [`build()`](Self::build) will panic if backend is not set.
    pub fn backend<NB>(self, backend: NB) -> CacheMiddlewareBuilder<NB, C, CM>
    where
        NB: CacheBackend,
    {
        CacheMiddlewareBuilder {
            backend: Some(Arc::new(backend)),
            configuration: self.configuration,
            concurrency_manager: self.concurrency_manager,
        }
    }

    /// Sets the cache configuration.
    ///
    /// Defaults to [`HttpEndpoint::default()`](hitbox_http::HttpEndpoint::default) if not called.
    pub fn config<NC>(self, configuration: NC) -> CacheMiddlewareBuilder<B, NC, CM> {
        CacheMiddlewareBuilder {
            backend: self.backend,
            configuration,
            concurrency_manager: self.concurrency_manager,
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
        }
    }

    /// Builds the cache middleware.
    ///
    /// # Panics
    ///
    /// Panics if [`backend()`](Self::backend) was not called.
    pub fn build(self) -> CacheMiddleware<B, C, CM> {
        CacheMiddleware {
            backend: self.backend.expect("backend is required"),
            configuration: self.configuration,
            concurrency_manager: self.concurrency_manager,
        }
    }
}

impl CacheMiddlewareBuilder<(), HttpEndpoint, NoopConcurrencyManager> {
    /// Creates a new builder. Equivalent to [`CacheMiddleware::builder()`].
    pub fn new() -> Self {
        Self {
            backend: None,
            configuration: HttpEndpoint::default(),
            concurrency_manager: NoopConcurrencyManager,
        }
    }
}

impl Default for CacheMiddlewareBuilder<(), HttpEndpoint, NoopConcurrencyManager> {
    fn default() -> Self {
        Self::new()
    }
}
