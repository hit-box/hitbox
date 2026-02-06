//! Upstream adapter for bridging Tower services to Hitbox.
//!
//! This module provides [`TowerUpstream`](crate::upstream::TowerUpstream), an adapter
//! that implements Hitbox's [`Upstream`] trait for Tower services. This allows the
//! caching layer to call the wrapped service on cache misses.
//!
//! Users typically don't interact with this module directly — it's used internally
//! by [`CacheService`](crate::service::CacheService).
//!
//! [`Upstream`]: hitbox_core::Upstream

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::ready;
use hitbox_core::Upstream;
use hitbox_http::{BufferedBody, CacheableHttpRequest, CacheableHttpResponse};
use http::{Request, Response};
use hyper::body::Body as HttpBody;
use pin_project::pin_project;
use tower::Service;

/// Future returned by [`TowerUpstream::call`].
///
/// Wraps the underlying service future and converts the response from
/// `Response<ResBody>` to [`CacheableHttpResponse<ResBody>`].
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It appears as the `Future` type
/// in [`TowerUpstream`]'s [`Upstream`] implementation.
///
/// [`Upstream`]: hitbox_core::Upstream
/// [`CacheableHttpResponse<ResBody>`]: hitbox_http::CacheableHttpResponse
#[pin_project]
pub struct TowerUpstreamFuture<F, ResBody, E> {
    #[pin]
    inner: F,
    _phantom: PhantomData<(ResBody, E)>,
}

impl<F, ResBody, E> TowerUpstreamFuture<F, ResBody, E> {
    /// Creates a new future wrapping the service's response future.
    pub fn new(inner: F) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<F, ResBody, E> Future for TowerUpstreamFuture<F, ResBody, E>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
    ResBody: HttpBody,
{
    type Output = Result<CacheableHttpResponse<ResBody>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match ready!(this.inner.poll(cx)) {
            Ok(response) => {
                let (parts, body) = response.into_parts();
                let buffered = Response::from_parts(parts, BufferedBody::Passthrough(body));
                Poll::Ready(Ok(CacheableHttpResponse::from_response(buffered)))
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

/// Adapter that implements Hitbox's [`Upstream`] trait for Tower services.
///
/// `TowerUpstream` bridges the gap between Tower's [`Service`] trait and Hitbox's
/// [`Upstream`] trait. It converts [`CacheableHttpRequest`] to `http::Request`
/// for the service call, and wraps responses in [`CacheableHttpResponse`].
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It's used internally by
/// [`CacheService`](crate::service::CacheService) to call the upstream service
/// on cache misses.
///
/// # Type Parameters
///
/// * `S` - The Tower service being adapted
/// * `ReqBody` - Request body type
/// * `ResBody` - Response body type
///
/// [`Upstream`]: hitbox_core::Upstream
/// [`Service`]: tower::Service
/// [`CacheableHttpRequest`]: hitbox_http::CacheableHttpRequest
/// [`CacheableHttpResponse`]: hitbox_http::CacheableHttpResponse
pub struct TowerUpstream<S, ReqBody, ResBody> {
    service: S,
    _phantom: PhantomData<(ReqBody, ResBody)>,
}

impl<S, ReqBody, ResBody> TowerUpstream<S, ReqBody, ResBody> {
    /// Creates a new upstream adapter wrapping the given service.
    pub fn new(service: S) -> Self {
        Self {
            service,
            _phantom: PhantomData,
        }
    }
}

impl<S, ReqBody, ResBody> Upstream<CacheableHttpRequest<ReqBody>>
    for TowerUpstream<S, ReqBody, ResBody>
where
    S: Service<Request<BufferedBody<ReqBody>>, Response = Response<ResBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Send,
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: Send,
    ResBody: HttpBody + Send + 'static,
{
    type Response = Result<CacheableHttpResponse<ResBody>, S::Error>;
    type Future<'a> = TowerUpstreamFuture<S::Future, ResBody, S::Error> where Self: 'a, CacheableHttpRequest<ReqBody>: 'a;

    fn call<'a>(&mut self, req: CacheableHttpRequest<ReqBody>) -> Self::Future<'a>
    where
        Self: 'a,
        CacheableHttpRequest<ReqBody>: 'a,
    {
        let http_req = req.into_request();
        let inner = self.service.call(http_req);
        TowerUpstreamFuture::new(inner)
    }
}
