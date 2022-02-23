use crate::{CacheError, CacheState, CacheableResponse, CachedValue};
use async_trait::async_trait;
pub use hitbox_backend::{EvictionPolicy, TtlSettings};

/// Type alias for backend or upstream operations in runtime adapter.
pub type AdapterResult<T> = Result<T, CacheError>;

/// Trait describes interaction with cache states (FSM) and cache backend.
///
/// Main idea of this trait is a separation of FSM transitions logic from
/// specific backend implementation.
#[async_trait]
pub trait RuntimeAdapter
where
    Self::UpstreamResult: CacheableResponse,
{
    /// Associated type describes the upstream result.
    type UpstreamResult;

    /// Send data to upstream and return [`Self::UpstreamResult`]
    async fn poll_upstream(&mut self) -> AdapterResult<Self::UpstreamResult>;

    /// Check cache and return current [state](`crate::CacheState`) of cached data.
    async fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>>;

    /// Write or update [`Self::UpstreamResult`] into cache.
    async fn update_cache<'a>(
        &self,
        cached_value: &'a CachedValue<Self::UpstreamResult>,
    ) -> AdapterResult<()>;

    /// Returns eviction settings for current cacheable data.
    fn eviction_settings(&self) -> EvictionPolicy;
}
