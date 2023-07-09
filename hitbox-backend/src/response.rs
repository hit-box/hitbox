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
    Self: Sized + Send + 'static,
    Self::Cached: Clone,
{
    type Cached;

    async fn cache_policy(
        self,
        predicates: &[Box<dyn Predicate<Self> + Send>],
    ) -> CachePolicy<Self> {
        let predicates_result = stream::iter(predicates)
            .fold(PredicateResult::Cacheable(self), PredicateResult::chain)
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

#[async_trait]
impl<T, E> CacheableResponse for Result<T, E>
where
    T: CacheableResponse + 'static,
    E: Send + 'static,
    T::Cached: Send,
{
    type Cached = <T as CacheableResponse>::Cached;

    async fn into_cached(self) -> Self::Cached {
        unimplemented!()
    }

    async fn from_cached(cached: Self::Cached) -> Self {
        unimplemented!()
    }
}
