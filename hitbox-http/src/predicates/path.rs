use std::marker::PhantomData;

use crate::CacheableHttpRequest;
use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::predicates::{Predicate, PredicateResult};

pub struct Path<P> {
    resource: ResourceDef,
    inner: P,
}

// impl<P, ReqBody> Path<P, ReqBody> {
//     pub fn new(value: String) -> Self {
//         Self(ResourceDef::new(value))
//     }
// }

pub trait PathPredicate<ReqBody>: Sized {
    fn path(self, resource: ResourceDef) -> Path<Self>;
}

impl<P, ReqBody> PathPredicate<ReqBody> for P
where
    P: Predicate<CacheableHttpRequest<ReqBody>>,
{
    fn path(self, resource: ResourceDef) -> Path<Self> {
        Path {
            resource,
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate<CacheableHttpRequest<ReqBody>> for Path<P>
where
    P: Predicate<CacheableHttpRequest<ReqBody>> + Sync,
    ReqBody: Send + 'static,
{
    async fn check(
        &self,
        request: CacheableHttpRequest<ReqBody>,
    ) -> PredicateResult<CacheableHttpRequest<ReqBody>> {
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
