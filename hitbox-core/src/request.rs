use async_trait::async_trait;

use crate::{
    CacheKey, CachePolicy,
    extractor::Extractor,
    predicate::{Predicate, PredicateError},
};

pub struct CacheablePolicyData<T> {
    pub key: CacheKey,
    pub request: T,
}

impl<T> CacheablePolicyData<T> {
    pub fn new(key: CacheKey, request: T) -> Self {
        CacheablePolicyData { key, request }
    }
}

pub type RequestCachePolicy<T> = CachePolicy<CacheablePolicyData<T>, T>;

#[async_trait]
pub trait CacheableRequest
where
    Self: Sized,
{
    async fn cache_policy<P, E>(
        self,
        predicates: P,
        extractors: E,
    ) -> Result<RequestCachePolicy<Self>, PredicateError>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync;
}
