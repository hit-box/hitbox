use async_trait::async_trait;
use hitbox::{
    predicate::{Predicate, PredicateResult},
    CacheablePolicyData, RequestCachePolicy, {CachePolicy, CacheableRequest, Extractor},
};
use http::{request::Parts, Request};
use hyper::body::HttpBody;

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
    async fn cache_policy<P, E>(self, predicates: P, extractors: E) -> RequestCachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync,
    {
        //dbg!("CacheableHttpRequest::cache_policy");
        let (request, key) = extractors.get(self).await.into_cache_key();

        match predicates.check(request).await {
            PredicateResult::Cacheable(request) => {
                CachePolicy::Cacheable(CacheablePolicyData { key, request })
            }
            PredicateResult::NonCacheable(request) => CachePolicy::NonCacheable(request),
        }
    }
}
