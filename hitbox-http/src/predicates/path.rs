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
impl Predicate<CacheableHttpRequest> for Path {
    async fn check(&self, request: CacheableHttpRequest) -> PredicateResult<CacheableHttpRequest> {
        if self.0.is_match(request.parts().uri.path()) {
            PredicateResult::Cacheable(request)
        } else {
            PredicateResult::NonCacheable(request)
        }
    }
}
