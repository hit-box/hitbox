use crate::{CacheError, CacheState, CacheableResponse, CachedValue};
use std::future::Future;
use std::pin::Pin;
pub use hitbox_backend::{EvictionPolicy, TtlSettings};

/// Type alias for backend or upstream operations in runtime adapter.
pub type AdapterResult<T> = Pin<Box<dyn Future<Output = Result<T, CacheError>>>>;

/// Trait describes interaction with cache states (FSM) and cache backend.
///
/// Main idea of this trait is a separation of FSM transitions logic from
/// specific backend implementation.
pub trait RuntimeAdapter
where
    Self::UpstreamResult: CacheableResponse,
{
    /// Associated type describes the upstream result.
    type UpstreamResult;

    /// Send data to upstream and return [`Self::UpstreamResult`]
    fn poll_upstream(&mut self) -> AdapterResult<Self::UpstreamResult>;

    /// Check cache and return current [state](`crate::CacheState`) of cached data.
    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>>;

    /// Write or update [`Self::UpstreamResult`] into cache.
    fn update_cache<'a>(&self, cached_value: &'a CachedValue<Self::UpstreamResult>) -> Pin<Box<dyn Future<Output = Result<(), CacheError>> + 'a>>;

    /// Returns eviction settings for current cacheable data.
    fn eviction_settings(&self) -> EvictionPolicy;
}
