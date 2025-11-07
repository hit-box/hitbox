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
    ReqBody: Send + 'static,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> Result<PredicateResult<Self::Subject>, hitbox::PredicateError> {
        match self.inner.check(request).await? {
            PredicateResult::Cacheable(request) => {
                if self.resource.is_match(request.parts().uri.path()) {
                    Ok(PredicateResult::Cacheable(request))
                } else {
                    Ok(PredicateResult::NonCacheable(request))
                }
            }
            PredicateResult::NonCacheable(request) => Ok(PredicateResult::NonCacheable(request)),
        }
    }
}
