use crate::CacheState;
use crate::runtime::{RuntimeAdapter, AdapterResult};
use crate::error::CacheError;

enum MockUpstreamState {
    Ok,
    Error,
}

enum MockCacheState {
    Actual,
    Stale,
    Miss,
    Error,
}

pub struct MockAdapter<T>
where
    T: Clone,
{
    upstream_value: T,
    upstream_state: MockUpstreamState,
    cache_state: MockCacheState,
}

impl<T> MockAdapter<T>
where
    T: Clone,
{
    pub fn build() -> MockAdapterBuilder<T> {
        MockAdapterBuilder {
            upstream_value: None,
            upstream_state: MockUpstreamState::Ok,
            cache_state: MockCacheState::Actual,
        }
    }
}

pub struct MockAdapterBuilder<T>
where
    T: Clone,
{
    upstream_value: Option<T>,
    upstream_state: MockUpstreamState,
    cache_state: MockCacheState,
}

impl<T> MockAdapterBuilder<T>
where
    T: Clone,
{
    pub fn with_upstream_value(self, value: T) -> Self {
        MockAdapterBuilder {
            upstream_value: Some(value),
            ..self
        }
    }
    pub fn with_upstream_error(self) -> Self {
        MockAdapterBuilder {
            upstream_state: MockUpstreamState::Error,
            ..self
        }
    }
    pub fn with_cache_actual(self) -> Self {
        MockAdapterBuilder {
            cache_state: MockCacheState::Actual,
            ..self
        }
    }
    pub fn with_cache_stale(self) -> Self {
        MockAdapterBuilder {
            cache_state: MockCacheState::Stale,
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
            upstream_value: self.upstream_value.expect("Upstream value cannot be empty"),
            upstream_state: self.upstream_state,
            cache_state: self.cache_state,
        }
    }
}

impl<T> RuntimeAdapter for MockAdapter<T>
where
    T: Clone + 'static,
{
    type UpstreamResult = T;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult> {
        let value = self.upstream_value.clone();
        let result = match self.upstream_state {
            MockUpstreamState::Ok => Ok(value),
            MockUpstreamState::Error => Err(CacheError::DeserializeError),
        };
        Box::pin(async { result })
    }
    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>> {
        Box::pin(async { Ok(CacheState::Miss) })
    }
}
