use std::fmt::Debug;

use async_trait::async_trait;
use chrono::Utc;
use futures::{stream, StreamExt};

use crate::{
    predicates::{Predicate, PredicateResult},
    CachedValue,
};

/// This trait determines which types should be cached or not.
pub enum CachePolicy<C>
where
    C: CacheableResponse,
{
    /// This variant should be stored in cache backend
    Cacheable(CachedValue<C::Cached>),
    /// This variant shouldn't be stored in the cache backend.
    NonCacheable(C),
}

#[derive(Debug, PartialEq, Eq)]
pub enum CacheState<Cached> {
    Stale(Cached),
    Actual(Cached),
}

#[async_trait]
pub trait CacheableResponse
where
    Self: Sized + Send,
    Self::Cached: Clone,
{
    type Cached;

    async fn cache_policy<P>(self, predicates: &[P]) -> CachePolicy<Self>
    where
        P: Predicate<Self> + Sync,
    {
        let predicates_result = stream::iter(predicates)
            .fold(PredicateResult::NonCacheable(self), PredicateResult::chain)
            .await;
        match predicates_result {
            PredicateResult::Cacheable(res) => {
                CachePolicy::Cacheable(CachedValue::new(res.into_cached().await, Utc::now()))
            }
            PredicateResult::NonCacheable(res) => CachePolicy::NonCacheable(res),
        }
    }

    async fn into_cached(self) -> Self::Cached;

    async fn from_cached(cached: Self::Cached) -> Self;
}
