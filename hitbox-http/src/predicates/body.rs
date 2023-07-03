use std::fmt::Debug;

use async_trait::async_trait;
use bytes::Bytes;
use hitbox::predicates::{Predicate, PredicateResult};
use http::Request;
use hyper::body::{to_bytes, HttpBody};

use crate::CacheableHttpRequest;

/// Test version of Body predicate.
/// FIX: Only for testing request's body consuming.
pub struct Body;

#[async_trait]
impl<ReqBody> Predicate<CacheableHttpRequest<ReqBody>> for Body
where
    ReqBody: HttpBody + Send + 'static,

    // debug bounds
    ReqBody::Error: Debug,
    ReqBody::Data: Send,
    ReqBody: From<Bytes>,
{
    async fn check(
        &self,
        request: CacheableHttpRequest<ReqBody>,
    ) -> PredicateResult<CacheableHttpRequest<ReqBody>> {
        let (parts, body) = request.into_parts();
        let payload = to_bytes(body).await.unwrap();
        if payload.len() <= 4 {
            let request = Request::from_parts(parts, ReqBody::from(payload));
            return PredicateResult::Cacheable(CacheableHttpRequest::from_request(request));
        } else {
            let request = Request::from_parts(parts, ReqBody::from(payload));
            return PredicateResult::NonCacheable(CacheableHttpRequest::from_request(request));
        }
    }
}
