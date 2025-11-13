use crate::CacheableHttpRequest;
use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};

#[derive(Debug)]
pub struct Path<P> {
    resource: ResourceDef,
    inner: P,
}

impl<P> Path<P> {
    pub fn new(inner: P, resource: ResourceDef) -> Self {
        Self { resource, inner }
    }
}

pub trait PathPredicate: Sized {
    fn path(self, resource: String) -> Path<Self>;
}

impl<P> PathPredicate for P
where
    P: Predicate,
{
    fn path(self, resource: String) -> Path<Self> {
        Path {
            resource: ResourceDef::from(resource),
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Path<P>
where
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                if self.resource.is_match(request.parts().uri.path()) {
                    PredicateResult::Cacheable(request)
                } else {
                    PredicateResult::NonCacheable(request)
                }
            }
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
