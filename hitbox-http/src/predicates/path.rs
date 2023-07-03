use crate::CacheableHttpRequest;
use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::predicates::{Predicate, PredicateResult};

pub struct Path(ResourceDef);

impl Path {
    pub fn new(value: String) -> Self {
        Self(ResourceDef::new(value))
    }
}

#[async_trait]
impl<ReqBody> Predicate<CacheableHttpRequest<ReqBody>> for Path
where
    ReqBody: Send + 'static,
{
    async fn check(
        &self,
        request: CacheableHttpRequest<ReqBody>,
    ) -> PredicateResult<CacheableHttpRequest<ReqBody>> {
        if self.0.is_match(request.parts().uri.path()) {
            PredicateResult::Cacheable(request)
        } else {
            PredicateResult::NonCacheable(request)
        }
    }
}
