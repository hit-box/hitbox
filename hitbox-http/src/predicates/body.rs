use std::fmt::Debug;

use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use http::Request;
use hyper::body::{to_bytes, HttpBody};

use crate::{CacheableHttpRequest, FromBytes};

/// Test version of Body predicate.
/// FIX: Only for testing request's body consuming.
pub struct Body<P> {
    inner: P,
}

pub trait BodyPredicate: Sized {
    fn body(self) -> Body<Self>;
}

impl<P> BodyPredicate for P
where
    P: Predicate,
{
    fn body(self) -> Body<Self> {
        Body { inner: self }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Body<P>
where
    ReqBody: HttpBody + Send + 'static,
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,

    // debug bounds
    ReqBody::Error: Debug,
    ReqBody::Data: Send,
    ReqBody: FromBytes,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let (parts, body) = request.into_parts();
                let payload = to_bytes(body).await.unwrap();
                //dbg!("BodyPredicate::check");
                //dbg!(&payload);
                if payload.len() <= 4 {
                    let request = Request::from_parts(parts, ReqBody::from_bytes(payload));
                    return PredicateResult::Cacheable(CacheableHttpRequest::from_request(request));
                } else {
                    let request = Request::from_parts(parts, ReqBody::from_bytes(payload));
                    return PredicateResult::NonCacheable(CacheableHttpRequest::from_request(
                        request,
                    ));
                }
            }
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
