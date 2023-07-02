use std::any::type_name;

use async_trait::async_trait;
use futures::{stream, StreamExt};
use hitbox::{
    cache::{CacheKey, CachePolicy, CacheableRequest},
    predicates::{Predicate, PredicateResult},
    Cacheable,
};
use http::{request::Parts, Request};
use hyper::body::{Body, HttpBody};

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
        let uri = self.parts.uri.clone();
        Request::builder()
            .uri(uri)
            // .body(Body::wrap_stream(stream))
            .body(self.body)
            .unwrap()
    }

    pub fn parts(&self) -> &Parts {
        &self.parts
    }
}

#[async_trait]
impl<ReqBody> CacheableRequest for CacheableHttpRequest<ReqBody>
where
    ReqBody: Send,
{
    async fn cache_policy<P>(self, predicates: &[P]) -> hitbox::cache::CachePolicy<Self>
    where
        P: Predicate<Self> + Send + Sync,
    {
        let predicate_result = stream::iter(predicates)
            .fold(PredicateResult::NonCacheable(self), PredicateResult::chain)
            .await;
        match predicate_result {
            PredicateResult::Cacheable(request) => CachePolicy::Cacheable(request),
            PredicateResult::NonCacheable(request) => CachePolicy::NonCacheable(request),
        }
    }
}
