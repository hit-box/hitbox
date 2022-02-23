//! Cacheable trait and implementation of cache logic.

use crate::CacheError;
use hitbox_backend::CacheableResponse;
#[cfg(feature = "derive")]
pub use hitbox_derive::Cacheable;

/// Trait describes cache configuration per type that implements this trait.
pub trait Cacheable {
    /// Method should return unique identifier for struct object.
    ///
    /// In cache storage it may prepends with cache version and Upstream name.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox::cache::Cacheable;
    /// use hitbox::CacheError;
    ///
    /// struct QueryNothing {
    ///     id: Option<i32>,
    /// }
    ///
    /// impl Cacheable for QueryNothing {
    ///     fn cache_key(&self) -> Result<String, CacheError> {
    ///         let key = format!("{}::id::{}", self.cache_key_prefix(), self.id.map_or_else(
    ///             || "None".to_owned(), |id| id.to_string())
    ///         );
    ///         Ok(key)
    ///     }
    ///     fn cache_key_prefix(&self) -> String { "database::QueryNothing".to_owned() }
    /// }
    ///
    /// let query = QueryNothing { id: Some(1) };
    /// assert_eq!(query.cache_key().unwrap(), "database::QueryNothing::id::1");
    /// let query = QueryNothing { id: None };
    /// assert_eq!(query.cache_key().unwrap(), "database::QueryNothing::id::None");
    /// ```
    fn cache_key(&self) -> Result<String, CacheError>;

    /// Method return cache key prefix based on message type.
    fn cache_key_prefix(&self) -> String;

    /// Describe time-to-live (ttl) value for cache storage in seconds.
    ///
    /// After that time value will be removed from cache storage.
    fn cache_ttl(&self) -> u32 {
        60
    }

    /// Describe expire\stale timeout value for cache storage in seconds.
    ///
    /// After that time cached value marked as stale.
    ///
    /// ```ignore
    /// |__cache_is_valid__|__cache_is_stale__| -> time
    ///                    ^                  ^
    ///                 stale_ttl       ttl (cache evicted)
    /// ```
    fn cache_stale_ttl(&self) -> u32 {
        let ttl = self.cache_ttl();
        let stale_time = 5;
        if ttl >= stale_time {
            ttl - stale_time
        } else {
            0
        }
    }

    /// Describe current cache version for this type.
    fn cache_version(&self) -> u32 {
        0
    }
}

pub enum CacheState {
    Running,
    Stopped,
}

pub struct Cache {
    state: CacheState
}

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

impl Cache {
    fn builder() -> CacheBuilder {
        CacheBuilder
    }

    async fn start(&self) {

    }

    async fn process<F, Req, Res, ResFuture>(&self, upstream: F, request: Req) -> Res
    where
        F: Fn(Req) -> ResFuture,
        ResFuture: Future<Output=Res>,
    {
        upstream(request).await
    }
}

pub struct CacheBuilder;

impl CacheBuilder {
    fn build() -> Cache {
        Cache { state: CacheState::Stopped }
    }
}

pub struct FutureAdapter<In, Out, U> {
    _response: PhantomData<Out>,
    request: In,
    upstream: U,
}

// impl<In, Out, U> crate::runtime::RuntimeAdapter for FutureAdapter<In, Out, U> 
// where
    // Out: CacheableResponse
// {
    // type UpstreamResult = Out; 
    // fn update_cache<'a>(&self, cached_value: &'a hitbox_backend::CachedValue<Self::UpstreamResult>) -> Pin<Box<dyn Future<Output = Result<(), CacheError>> + 'a>> {
        
    // }

    // fn poll_cache(&self) -> crate::runtime::AdapterResult<crate::CacheState<Self::UpstreamResult>> {
        
    // }

    // fn poll_upstream(&mut self) -> crate::runtime::AdapterResult<Self::UpstreamResult> {
        // Ok(self.upstream.await?)
    // }

    // fn eviction_settings(&self) -> hitbox_backend::EvictionPolicy {
        // hitbox_backend::EvictionPolicy::Ttl(hitbox_backend::TtlSettings{ ttl: 42, stale_ttl: 24 }) 
    // }
// }

#[cfg(test)]
mod tests {
    use super::*;

    struct Message(i32);

    impl Cacheable for Message {
        fn cache_key(&self) -> Result<String, CacheError> {
            Ok("Message".to_owned())
        }
        fn cache_key_prefix(&self) -> String {
            "Message".to_owned()
        }
        fn cache_ttl(&self) -> u32 {
            2
        }
    }

    #[test]
    fn test_cache_stale_ttl_subtract_overflow() {
        let a = Message(42);
        assert_eq!(0, a.cache_stale_ttl());
    }

    async fn upstream_fn(message: Message) -> i32 {
        message.0 
    }

    #[tokio::test]
    async fn test_cache_process() {
        let cache = Cache { state: CacheState::Running };
        let response = cache.process(upstream_fn, Message(42)).await;
        dbg!(response);
    }
}
