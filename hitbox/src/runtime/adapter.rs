use crate::response::CacheableResponse;
use crate::{CacheError, CacheState, CachedValue};
use std::future::Future;
use std::pin::Pin;

/// Type alias for backend or upstream operations in runtime adapter.
pub type AdapterResult<T> = Pin<Box<dyn Future<Output = Result<T, CacheError>>>>;

/// TTL eviction settings.
///
/// More information you cat see in [`crate::Cacheable`] trait implementation.
pub struct TtlSettings {
    /// Describe current cached data TTL value.
    ///
    /// More information you can see in [`crate::Cacheable::cache_ttl`].
    pub ttl: u32,

    /// Describe current cached data stale TTL value.
    ///
    /// More information you can see in [`crate::Cacheable::cache_stale_ttl`].
    pub stale_ttl: u32,
}

/// Cached data eviction policy settings.
pub enum EvictionPolicy {
    /// Eviction by TTL settings.
    Ttl(TtlSettings),
}

pub trait RuntimeAdapter
where
    Self::UpstreamResult: CacheableResponse,
{
    type UpstreamResult;
    fn poll_upstream(&mut self) -> AdapterResult<Self::UpstreamResult>;
    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>>;
    fn update_cache(&self, cached_value: &CachedValue<Self::UpstreamResult>) -> AdapterResult<()>;
    fn eviction_settings(&self) -> EvictionPolicy;
}
