use async_trait::async_trait;
use futures::{stream, StreamExt};
use hitbox::{
    cache::{CacheKey, CachePolicy, CacheableRequest, Selector},
    predicates::{Predicate, PredicateResult},
    Cacheable,
};
use http::{request::Parts, Request};
use hyper::body::{Body, HttpBody};

#[derive(Debug)]
pub struct CacheableHttpRequest<ReqBody> {
    parts: Parts,
    body: ReqBody,
}

impl<ReqBody> CacheableHttpRequest<ReqBody> {
    pub fn from_request(request: Request<ReqBody>) -> Self
    where
        ReqBody: HttpBody,
    {
        let (parts, body) = request.into_parts();
        Self { parts, body }
    }

    pub fn into_request(self) -> Request<ReqBody> {
        Request::from_parts(self.parts, self.body)
    }

    pub fn parts(&self) -> &Parts {
        &self.parts
    }

    pub fn into_parts(self) -> (Parts, ReqBody) {
        (self.parts, self.body)
    }
}

#[async_trait]
impl<ReqBody> CacheableRequest for CacheableHttpRequest<ReqBody>
where
    ReqBody: Send + 'static,
{
    async fn cache_policy<P>(self, predicates: P) -> hitbox::cache::CachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
    {
        dbg!("CacheableHttpRequest::cache_policy");
        match predicates.check(self).await {
            PredicateResult::Cacheable(request) => CachePolicy::Cacheable(request),
            PredicateResult::NonCacheable(request) => CachePolicy::NonCacheable(request),
        }
    }
}
