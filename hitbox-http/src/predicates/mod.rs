use async_trait::async_trait;
use hitbox_backend::predicates::{Predicate, PredicateResult};

use crate::CacheableHttpRequest;

pub mod body;
pub mod header;
pub mod path;
pub mod query;

pub struct NeutralPredicate;

impl NeutralPredicate {
    pub fn new() -> Self {
        NeutralPredicate
    }
}

#[async_trait]
impl<ReqBody> Predicate<CacheableHttpRequest<ReqBody>> for NeutralPredicate
where
    ReqBody: Send + 'static,
{
    async fn check(
        &self,
        subject: CacheableHttpRequest<ReqBody>,
    ) -> PredicateResult<CacheableHttpRequest<ReqBody>> {
        PredicateResult::Cacheable(subject)
    }
}
