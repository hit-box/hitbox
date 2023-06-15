use std::fmt::Debug;

use async_trait::async_trait;
use chrono::Utc;

use crate::CachedValue;

pub enum CachePolicy<Cached> {
    Cacheable(Cached),
    NonCacheable,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CacheState<Cached> {
    Stale(Cached),
    Actual(Cached),
}

#[async_trait]
pub trait CacheableResponseWrapper {
    type Source;
    type Serializable;
    type Error: Debug;

    fn from_source(source: Self::Source) -> Self;
    fn into_source(self) -> Self::Source;
    fn from_serializable(serializable: Self::Serializable) -> Self;
    async fn into_serializable(self) -> Result<Self::Serializable, Self::Error>;
}

#[async_trait]
pub trait CacheableResponse: Sized + CacheableResponseWrapper
where
    Self: CacheableResponseWrapper<Serializable = Self::Cached> + Send,
{
    type Cached;

    async fn cache_policy(self) -> CachePolicy<CachedValue<Self::Cached>> {
        if self.is_cacheable() {
            let cached = self.into_serializable().await.unwrap();
            let cached_value = CachedValue::new(cached, Utc::now());
            CachePolicy::Cacheable(cached_value)
        } else {
            CachePolicy::NonCacheable
        }
    }

    fn from_cached(cached: CachedValue<Self::Cached>) -> CacheState<Self> {
        // TODO: check stale state
        CacheState::Actual(Self::from_serializable(cached.into_inner()))
    }

    fn is_cacheable(&self) -> bool;
}
