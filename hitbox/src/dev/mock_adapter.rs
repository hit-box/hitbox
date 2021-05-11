use crate::error::CacheError;
use crate::runtime::{AdapterResult, RuntimeAdapter, EvictionPolicy, TtlSettings};
use crate::value::{CacheState, CachedValue};
use crate::CacheableResponse;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Clone, Debug)]
enum MockUpstreamState<T> {
    Ok(T),
    Error,
}

#[derive(Clone, Debug)]
enum MockCacheState<T> {
    Actual(T),
    Stale((T, DateTime<Utc>)),
    Miss,
    Error,
}

#[derive(Clone, Debug)]
pub struct MockAdapter<T>
where
    T: Clone,
{
    upstream_state: MockUpstreamState<T>,
    cache_state: MockCacheState<T>,
}

impl<T> MockAdapter<T>
where
    T: Clone,
{
    pub fn build() -> MockAdapterBuilder<T> {
        MockAdapterBuilder {
            upstream_state: MockUpstreamState::Error,
            cache_state: MockCacheState::Error,
        }
    }
}

pub struct MockAdapterBuilder<T>
where
    T: Clone,
{
    upstream_state: MockUpstreamState<T>,
    cache_state: MockCacheState<T>,
}

impl<T> MockAdapterBuilder<T>
where
    T: Clone,
{
    pub fn with_upstream_value(self, value: T) -> Self {
        MockAdapterBuilder {
            upstream_state: MockUpstreamState::Ok(value),
            ..self
        }
    }
    pub fn with_upstream_error(self) -> Self {
        MockAdapterBuilder {
            upstream_state: MockUpstreamState::Error,
            ..self
        }
    }
    pub fn with_cache_actual(self, value: T) -> Self {
        MockAdapterBuilder {
            cache_state: MockCacheState::Actual(value),
            ..self
        }
    }
    pub fn with_cache_stale(self, value: T, expired: DateTime<Utc>) -> Self {
        MockAdapterBuilder {
            cache_state: MockCacheState::Stale((value, expired)),
            ..self
        }
    }
    pub fn with_cache_miss(self) -> Self {
        MockAdapterBuilder {
            cache_state: MockCacheState::Miss,
            ..self
        }
    }
    pub fn with_cache_error(self) -> Self {
        MockAdapterBuilder {
            cache_state: MockCacheState::Error,
            ..self
        }
    }
    pub fn finish(self) -> MockAdapter<T> {
        MockAdapter {
            upstream_state: self.upstream_state,
            cache_state: self.cache_state,
        }
    }
}

impl<T> RuntimeAdapter for MockAdapter<T>
where
    T: Clone + CacheableResponse + 'static,
{
    type UpstreamResult = T;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult> {
        let result = match self.clone().upstream_state {
            MockUpstreamState::Ok(value) => Ok(value),
            MockUpstreamState::Error => Err(CacheError::DeserializeError),
        };
        Box::pin(async { result })
    }

    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>> {
        let result = match self.clone().cache_state {
            MockCacheState::Actual(value) => Ok(CacheState::Actual(CachedValue::new(
                value,
                chrono::Utc::now(),
            ))),
            MockCacheState::Stale(value) => {
                Ok(CacheState::Stale(CachedValue::new(value.0, value.1)))
            }
            MockCacheState::Miss => Ok(CacheState::Miss),
            MockCacheState::Error => Err(CacheError::DeserializeError),
        };
        Box::pin(async { result })
    }

    fn update_cache(&self, cached_value: &CachedValue<Self::UpstreamResult>) -> AdapterResult<()> {
        Box::pin(async { Ok(()) })
    }

    fn eviction_settings(&self) -> EvictionPolicy {
        EvictionPolicy::Ttl(TtlSettings { ttl: 0, stale_ttl: 0 })
    }
}
