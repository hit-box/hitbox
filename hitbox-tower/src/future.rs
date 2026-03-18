//! Future types for the cache service.
//!
//! This module provides [`CacheServiceFuture`](crate::future::CacheServiceFuture),
//! the future returned by [`CacheService::call`]. It wraps the inner cache future
//! and adds cache status headers to responses.
//!
//! Users typically don't interact with this module directly.
//!
//! [`CacheService::call`]: crate::service::CacheService

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Future;
use futures::ready;
use hitbox::{CacheContext, CacheStatus, CacheStatusExt};
use hitbox_http::{BufferedBody, CacheableHttpResponse, HttpCacheData, HttpCacheStatusConfig};
use http::Response;
use pin_project::pin_project;

/// Future returned by [`CacheService::call`](crate::service::CacheService).
///
/// This future wraps the inner `CacheFuture` and performs the final transformation:
/// converting [`CacheableHttpResponse`] to `http::Response` and adding the cache
/// status header (`HIT`/`MISS`/`STALE`).
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It's the `Future` type returned when
/// calling the [`CacheService`](crate::service::CacheService) as a Tower service.
///
/// # Type Parameters
///
/// * `F` - The inner future (typically `CacheFuture`)
/// * `ResBody` - Response body type
/// * `E` - Error type from the upstream service
///
/// [`CacheableHttpResponse`]: hitbox_http::CacheableHttpResponse
#[pin_project]
pub struct CacheServiceFuture<F, ResBody, E>
where
    F: Future<Output = (Result<CacheableHttpResponse<ResBody>, E>, CacheContext)>,
    ResBody: hyper::body::Body,
{
    #[pin]
    inner: F,
    cache_status_config: HttpCacheStatusConfig,
}

impl<F, ResBody, E> CacheServiceFuture<F, ResBody, E>
where
    F: Future<Output = (Result<CacheableHttpResponse<ResBody>, E>, CacheContext)>,
    ResBody: hyper::body::Body,
{
    /// Creates a new future that will add cache status headers to the response.
    pub fn new(inner: F, cache_status_config: HttpCacheStatusConfig) -> Self {
        Self {
            inner,
            cache_status_config,
        }
    }
}

impl<F, ResBody, E> Future for CacheServiceFuture<F, ResBody, E>
where
    F: Future<Output = (Result<CacheableHttpResponse<ResBody>, E>, CacheContext)>,
    ResBody: hyper::body::Body,
{
    type Output = Result<Response<BufferedBody<ResBody>>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // Poll the inner CacheFuture
        let (result, cache_context) = ready!(this.inner.poll(cx));

        // Transform the response and add cache headers
        let response = result.map(|mut cacheable_response| {
            // Set HTTP-specific extension data (upstream status code)
            let http_ext = if matches!(cache_context.status, CacheStatus::Forward(_)) {
                Some(HttpCacheData {
                    upstream_status: cacheable_response.parts.status.as_u16(),
                })
            } else {
                None
            };
            let http_ctx = cache_context.with_extensions(http_ext);

            // Add cache status headers (RFC 9211 Cache-Status, Age, legacy x-cache-status)
            cacheable_response.cache_status(&http_ctx, this.cache_status_config);

            cacheable_response.into_response()
        });

        Poll::Ready(response)
    }
}
