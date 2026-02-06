//! Tower service implementation for HTTP caching.
//!
//! This module provides [`CacheService`](crate::service::CacheService), the Tower
//! `Service` that performs the actual caching logic. Users typically don't construct
//! this directly — it's created by the [`Cache`](crate::Cache) layer.

use std::{future::Future, pin::Pin, sync::Arc};

use hitbox::concurrency::ConcurrencyManager;
use hitbox::config::{CacheConfigRouter, RouteMatch};
use hitbox::{CacheContext, backend::CacheBackend, fsm::CacheFuture};
use hitbox_core::{DisabledOffload, Offload, Upstream};
use hitbox_http::{BufferedBody, CacheableHttpRequest, CacheableHttpResponse};
use http::header::HeaderName;
use http::{Request, Response};
use hyper::body::Body as HttpBody;
use tower::Service;

use crate::future::CacheServiceFuture;
use crate::upstream::TowerUpstream;

/// Tower [`Service`] that wraps an upstream service with caching.
///
/// `CacheService` intercepts HTTP requests, checks the cache, and either
/// returns cached responses or forwards requests to the upstream service.
/// It adds a cache status header (`HIT`/`MISS`/`STALE`) to every response.
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It's produced when you apply
/// a [`Cache`] layer to a service via [`tower::ServiceBuilder`].
///
/// # Type Parameters
///
/// * `S` - The upstream Tower service being wrapped
/// * `B` - Cache backend (e.g., [`MokaBackend`])
/// * `C` - Configuration with predicates, extractors, and policy
/// * `CM` - Concurrency manager for dogpile prevention
/// * `O` - Offload strategy for background revalidation
///
/// [`Cache`]: crate::Cache
/// [`Service`]: tower::Service
/// [`MokaBackend`]: hitbox_moka::MokaBackend
pub struct CacheService<S, B, C, CM, O = DisabledOffload> {
    upstream: S,
    backend: Arc<B>,
    configuration: C,
    offload: O,
    concurrency_manager: CM,
    cache_status_header: HeaderName,
}

impl<S, B, C, CM, O> CacheService<S, B, C, CM, O> {
    /// Creates a new cache service wrapping the given upstream.
    ///
    /// Prefer using [`Cache::builder()`] and [`tower::ServiceBuilder`] instead
    /// of constructing this directly.
    ///
    /// [`Cache::builder()`]: crate::Cache::builder
    pub fn new(
        upstream: S,
        backend: Arc<B>,
        configuration: C,
        offload: O,
        concurrency_manager: CM,
        cache_status_header: HeaderName,
    ) -> Self {
        CacheService {
            upstream,
            backend,
            configuration,
            offload,
            concurrency_manager,
            cache_status_header,
        }
    }
}

impl<S, B, C, CM, O> Clone for CacheService<S, B, C, CM, O>
where
    S: Clone,
    B: Clone,
    C: Clone,
    CM: Clone,
    O: Clone,
{
    fn clone(&self) -> Self {
        Self {
            upstream: self.upstream.clone(),
            backend: self.backend.clone(),
            configuration: self.configuration.clone(),
            offload: self.offload.clone(),
            concurrency_manager: self.concurrency_manager.clone(),
            cache_status_header: self.cache_status_header.clone(),
        }
    }
}

impl<S, B, C, CM, O, ReqBody, ResBody> Service<Request<ReqBody>> for CacheService<S, B, C, CM, O>
where
    S: Service<Request<BufferedBody<ReqBody>>, Response = Response<ResBody>>
        + Clone
        + Send
        + 'static,
    B: CacheBackend + Clone + Send + Sync + 'static,
    S::Future: Send,
    C: CacheConfigRouter<CacheableHttpRequest<ReqBody>, CacheableHttpResponse<ResBody>>
        + Clone
        + Send
        + Sync
        + 'static,
    CM: ConcurrencyManager<Result<CacheableHttpResponse<ResBody>, S::Error>> + Clone + 'static,
    O: Offload<'static> + Clone + 'static,
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: Send,
    ResBody: HttpBody + Send + 'static,
    ResBody::Error: Send,
    ResBody::Data: Send,
    S::Error: Send,
{
    type Response = Response<BufferedBody<ResBody>>;
    type Error = S::Error;
    type Future = CacheServiceFuture<
        Pin<
            Box<
                dyn Future<
                        Output = (
                            Result<CacheableHttpResponse<ResBody>, S::Error>,
                            CacheContext,
                        ),
                    > + Send,
            >,
        >,
        ResBody,
        S::Error,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Convert incoming Request<ReqBody> to CacheableHttpRequest<ReqBody>
        let (parts, body) = req.into_parts();
        let buffered_request = Request::from_parts(parts, BufferedBody::Passthrough(body));
        let cacheable_req = CacheableHttpRequest::from_request(buffered_request);

        let backend = self.backend.clone();
        let configuration = self.configuration.clone();
        let offload = self.offload.clone();
        let concurrency_manager = self.concurrency_manager.clone();
        let upstream = self.upstream.clone();

        let future = Box::pin(async move {
            match configuration.route(cacheable_req).await {
                RouteMatch::Matched {
                    request,
                    request_predicates,
                    response_predicates,
                    extractors,
                    policy,
                } => {
                    let upstream = TowerUpstream::new(upstream);
                    CacheFuture::new(
                        backend,
                        request,
                        upstream,
                        request_predicates,
                        response_predicates,
                        extractors,
                        Arc::new(policy),
                        offload,
                        concurrency_manager,
                    )
                    .await
                }
                RouteMatch::Miss(request) => {
                    let mut upstream = TowerUpstream::new(upstream);
                    let response = upstream.call(request).await;
                    (response, CacheContext::default())
                }
            }
        });

        // Wrap in CacheServiceFuture to add cache headers
        CacheServiceFuture::new(future, self.cache_status_header.clone())
    }
}
