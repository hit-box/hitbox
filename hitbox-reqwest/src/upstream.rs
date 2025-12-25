//! Upstream wrapper for reqwest-middleware's Next type.
//!
//! This module provides [`ReqwestUpstream`] which bridges the gap between
//! hitbox's [`Upstream`] trait and reqwest-middleware's [`Next`] type.
//!
//! # Overview
//!
//! When the cache middleware needs to fetch data from the actual HTTP endpoint
//! (on cache miss or stale), it uses [`ReqwestUpstream`] to:
//!
//! 1. Convert [`CacheableHttpRequest`] back to [`reqwest::Request`]
//! 2. Call the next middleware in the chain via [`Next::run`]
//! 3. Convert [`reqwest::Response`] to [`CacheableHttpResponse`]
//!
//! [`Upstream`]: hitbox_core::Upstream
//! [`Next`]: reqwest_middleware::Next
//! [`CacheableHttpRequest`]: hitbox_http::CacheableHttpRequest
//! [`CacheableHttpResponse`]: hitbox_http::CacheableHttpResponse

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use hitbox_core::Upstream;
use hitbox_http::{BufferedBody, CacheableHttpRequest, CacheableHttpResponse};
use http::Extensions;
use reqwest_middleware::{Next, Result};

/// Upstream wrapper that bridges reqwest-middleware's [`Next`] to hitbox's [`Upstream`] trait.
///
/// This adapter allows the hitbox cache FSM to call the remaining middleware
/// chain when it needs to fetch fresh data from upstream.
///
/// # Type Parameter
///
/// The lifetime `'a` comes from [`Next<'a>`], representing the middleware
/// chain's lifetime. This is why [`DisabledOffload`] is used in the middleware -
/// we cannot spawn background tasks with non-`'static` lifetimes.
///
/// [`Next`]: reqwest_middleware::Next
/// [`DisabledOffload`]: hitbox_core::DisabledOffload
pub struct ReqwestUpstream<'a> {
    next: Next<'a>,
    extensions: Extensions,
}

impl<'a> ReqwestUpstream<'a> {
    /// Creates a new upstream wrapper. Typically called internally by
    /// [`CacheMiddleware`](crate::CacheMiddleware).
    pub fn new(next: Next<'a>, extensions: Extensions) -> Self {
        Self { next, extensions }
    }
}

/// Implementation of [`Upstream`] for reqwest-middleware integration.
///
/// This allows the hitbox cache FSM to treat the remaining middleware chain
/// as an upstream service that can be called on cache misses.
///
/// [`Upstream`]: hitbox_core::Upstream
impl<'a> Upstream<CacheableHttpRequest<reqwest::Body>> for ReqwestUpstream<'a> {
    type Response = Result<CacheableHttpResponse<reqwest::Body>>;
    type Future = Pin<Box<dyn Future<Output = Self::Response> + Send + 'a>>;

    fn call(&mut self, req: CacheableHttpRequest<reqwest::Body>) -> Self::Future {
        let next = self.next.clone();
        let mut extensions = std::mem::take(&mut self.extensions);

        Box::pin(async move {
            // Convert CacheableHttpRequest back to reqwest::Request
            let http_request = req.into_request();
            let (parts, buffered_body) = http_request.into_parts();

            // Convert BufferedBody back to reqwest::Body
            let body = buffered_body_to_reqwest(buffered_body);

            // Reconstruct http::Request and convert to reqwest::Request
            let http_request = http::Request::from_parts(parts, body);
            let reqwest_request: reqwest::Request = http_request
                .try_into()
                .map_err(|e: reqwest::Error| reqwest_middleware::Error::Reqwest(e))?;

            // Call the next middleware
            let response = next.run(reqwest_request, &mut extensions).await?;

            // Convert reqwest::Response to CacheableHttpResponse
            let http_response: http::Response<reqwest::Body> = response.into();
            let (parts, body) = http_response.into_parts();
            let buffered_body = BufferedBody::Passthrough(body);
            let http_response = http::Response::from_parts(parts, buffered_body);

            Ok(CacheableHttpResponse::from_response(http_response))
        })
    }
}

/// Converts a [`BufferedBody`] to [`reqwest::Body`].
///
/// # Performance
///
/// This conversion is cheap for most cases:
///
/// - **Passthrough**: Unwraps the inner body with zero overhead
/// - **Complete**: Creates a body from the buffered bytes
/// - **Partial**: Wraps a [`PartialBufferedBody`] which implements [`http_body::Body`],
///   yielding the buffered prefix first, then the remaining stream
///
/// [`BufferedBody`]: hitbox_http::BufferedBody
/// [`PartialBufferedBody`]: hitbox_http::PartialBufferedBody
pub fn buffered_body_to_reqwest(buffered: BufferedBody<reqwest::Body>) -> reqwest::Body {
    match buffered {
        BufferedBody::Passthrough(body) => body,
        BufferedBody::Complete(Some(bytes)) => reqwest::Body::from(bytes),
        BufferedBody::Complete(None) => reqwest::Body::from(Bytes::new()),
        BufferedBody::Partial(partial) => {
            // PartialBufferedBody implements HttpBody, handling:
            // - prefix bytes (yielded first)
            // - remaining stream OR error
            reqwest::Body::wrap(partial)
        }
    }
}
