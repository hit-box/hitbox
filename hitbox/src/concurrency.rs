use std::future::Future;
use std::pin::Pin;

use crate::CacheKey;

/// Result of concurrency check - whether to proceed with upstream call or await existing response
pub enum ConcurrencyDecision<Res> {
    /// Proceed with the upstream call
    Proceed,
    /// Await response from another in-flight request
    Await(Pin<Box<dyn Future<Output = Res> + Send>>),
}

/// Trait for managing concurrent requests to prevent dogpile effect
pub trait ConcurrencyManager<Res>: Send + Sync {
    /// Check if this request should proceed to upstream or await an existing request
    fn check(&self, cache_key: &CacheKey) -> ConcurrencyDecision<Res>;

    /// Notify waiting requests that the response is ready and return it back
    fn complete(&self, cache_key: &CacheKey, response: Res) -> Res;
}

/// No-op implementation that always allows requests to proceed
pub struct NoopConcurrencyManager;

impl<Res> ConcurrencyManager<Res> for NoopConcurrencyManager
where
    Res: Send + 'static,
{
    fn check(&self, _cache_key: &CacheKey) -> ConcurrencyDecision<Res> {
        ConcurrencyDecision::Proceed
    }

    fn complete(&self, _cache_key: &CacheKey, response: Res) -> Res {
        // No-op: just return the response
        response
    }
}
