use std::fmt::Debug;

use async_trait::async_trait;
use chrono::Utc;

use crate::{
    predicate::{Predicate, PredicateResult},
    value::CacheValue,
    CachePolicy, EntityPolicyConfig,
};

/// This trait determines which types should be cached or not.
// pub enum CachePolicy<C>
// where
//     C: CacheableResponse,
// {
//     /// This variant should be stored in cache backend
//     Cacheable(CachedValue<C::Cached>),
//     /// This variant shouldn't be stored in the cache backend.
//     NonCacheable(C),
// }
pub type ResponseCachePolicy<C> = CachePolicy<CacheValue<<C as CacheableResponse>::Cached>, C>;

#[derive(Debug, PartialEq, Eq)]
pub enum CacheState<Cached> {
    Stale(Cached),
    Actual(Cached),
    Expired(Cached),
}

#[async_trait]
pub trait CacheableResponse
where
    Self: Sized + Send + 'static,
    Self::Cached: Clone,
{
    type Cached;
    type Subject: CacheableResponse;

    async fn cache_policy<P>(
        self,
        predicates: P,
        config: &EntityPolicyConfig,
    ) -> ResponseCachePolicy<Self>
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync;

    async fn into_cached(self) -> CachePolicy<Self::Cached, Self>;

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
    type Subject = T;

    async fn cache_policy<P>(
        self,
        predicates: P,
        config: &EntityPolicyConfig,
    ) -> ResponseCachePolicy<Self>
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync,
    {
        match self {
            Ok(response) => match predicates.check(response).await {
                PredicateResult::Cacheable(cacheable) => match cacheable.into_cached().await {
                    CachePolicy::Cacheable(res) => CachePolicy::Cacheable(CacheValue::new(
                        res,
                        config.ttl.map(|duration| Utc::now() + duration),
                        config.stale_ttl.map(|duration| Utc::now() + duration),
                    )),
                    CachePolicy::NonCacheable(res) => CachePolicy::NonCacheable(Ok(res)),
                },
                PredicateResult::NonCacheable(res) => CachePolicy::NonCacheable(Ok(res)),
            },
            Err(error) => ResponseCachePolicy::NonCacheable(Err(error)),
        }
    }

    async fn into_cached(self) -> CachePolicy<Self::Cached, Self> {
        match self {
            Ok(response) => match response.into_cached().await {
                CachePolicy::Cacheable(res) => CachePolicy::Cacheable(res),
                CachePolicy::NonCacheable(res) => CachePolicy::NonCacheable(Ok(res)),
            },
            Err(error) => CachePolicy::NonCacheable(Err(error)),
        }
    }

    async fn from_cached(cached: Self::Cached) -> Self {
        Ok(T::from_cached(cached).await)
    }
}
