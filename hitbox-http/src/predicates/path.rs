use std::marker::PhantomData;

use crate::CacheableHttpRequest;
use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::predicates::{Predicate, PredicateResult};

pub struct Path<P> {
    resource: ResourceDef,
    inner: P,
}

pub trait PathPredicate: Sized {
    fn path(self, resource: ResourceDef) -> Path<Self>;
}

impl<P> PathPredicate for P
where
    P: Predicate,
{
    fn path(self, resource: ResourceDef) -> Path<Self> {
        Path {
            resource,
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Path<P>
where
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: Send + 'static,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => match self.inner.check(request).await {
                PredicateResult::Cacheable(request) => {
                    if self.resource.is_match(request.parts().uri.path()) {
                        PredicateResult::Cacheable(request)
                    } else {
                        PredicateResult::NonCacheable(request)
                    }
                }
                PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
            },
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
