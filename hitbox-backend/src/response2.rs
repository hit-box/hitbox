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

pub trait CacheableResponse: Sized {
    type Cached;

    fn cache_policy<'a>(&'a self) -> CachePolicy<CachedValue<Self::Cached>>
    where
        Self::Cached: From<&'a Self>,
    {
        if self.is_cacheable() {
            let cached = self.into();
            let cached_value = CachedValue::new(cached, Utc::now());
            CachePolicy::Cacheable(cached_value)
        } else {
            CachePolicy::NonCacheable
        }
    }

    fn from_cached(cached: CachedValue<Self::Cached>) -> CacheState<Self>
    where
        Self: From<Self::Cached>,
    {
        // TODO: check stale state
        CacheState::Actual(Self::from(cached.into_inner()))
    }

    fn is_cacheable(&self) -> bool;
}
