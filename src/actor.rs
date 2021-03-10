//! Cache actor and Builder.
use std::marker::PhantomData;

use actix::prelude::*;
use actix_cache_backend::Backend;
use actix_cache_redis::RedisBackend;
use log::{debug, info, warn};

use crate::CacheError;
#[cfg(feature = "metrics")]
use crate::metrics::{
    CACHE_HIT_COUNTER, CACHE_MISS_COUNTER, CACHE_STALE_COUNTER, CACHE_UPSTREAM_HANDLING_HISTOGRAM,
};

pub enum StalePolicy {
    Disabled,
    Enabled,
}

impl Default for StalePolicy {
    fn default() -> StalePolicy {
        StalePolicy::Disabled
    }
}

pub enum CacheLockPolicy {
    Disabled,
    Enabled,
}

impl Default for CacheLockPolicy {
    fn default() -> CacheLockPolicy {
        CacheLockPolicy::Enabled
    }
}

/// Actix actor implements cache logic.
///
/// This actor implement only `Handler<QueryCache>`.
/// Where [QueryCache] - Actix message with two fields:
/// * Generic actix message for sending to upstream actor.
/// * Address of upstream actor
///
/// # Example
/// ```rust
/// use actix::prelude::*;
/// use actix_cache::{Cache, RedisBackend, CacheError};
///
/// #[actix_rt::main]
/// async fn main() -> Result<(), CacheError> {
///     let cache = Cache::new().await?.start();
///     Ok(())
/// }
/// ```
///
/// [QueryCache]: ../cache/struct.QueryCache.html
pub struct CacheActor<B>
where
    B: Backend,
{
    pub(crate) enabled: bool,
    pub(crate) stale_cache_policy: StalePolicy,
    pub(crate) cache_lock_policy: CacheLockPolicy,
    pub(crate) backend: Addr<B>,
}

use actix::dev::{ToEnvelope, MessageResponse};
use serde::de::DeserializeOwned;
use crate::response::{CacheableResponse, CachePolicy};
use crate::{cache::CachedValue, Cacheable, dev::{Get, Set, Lock, Delete, LockStatus}, QueryCache};

impl<B> CacheActor<B>
where
    B: Actor + Backend,
    <B as Actor>::Context:
        ToEnvelope<B, Get> + ToEnvelope<B, Set> + ToEnvelope<B, Lock> + ToEnvelope<B, Delete>,
{
    /// Initialize new Cache actor with default [RedisBackend].
    ///
    /// [RedisBackend]: ../../actix_cache_redis/actor/struct.RedisActor.html
    pub async fn new() -> Result<CacheActor<RedisBackend>, CacheError> {
        let backend = RedisBackend::new()
            .await
            .map_err(|err| CacheError::BackendError(err.into()))?
            .start();
        Ok(CacheBuilder::default().build(backend))
    }

    /// Creates new [CacheBuilder](struct.CacheBuilder.html) instance for Cache actor configuration.
    pub fn builder() -> CacheBuilder<B> {
        CacheBuilder::default()
    }

    pub(crate) async fn handle_actual<A, M>(msg: QueryCache<A, M>, cached: M::Result) -> Result<M::Result, CacheError>
    where
        M: Message + Send + Cacheable,
        M::Result: MessageResponse<A, M> + Send,
        A: Actor + Handler<M>,
    {
        #[cfg(feature = "metrics")] {
            let (actor, message) = (msg.upstream_name(), msg.message.cache_key_prefix());
            CACHE_HIT_COUNTER
                .with_label_values(&[&message, actor])
                .inc();
        }
        Ok(cached)
    }

    pub(crate) async fn handle_stale<A, S, M>(msg: QueryCache<A, M>, backend: &Addr<S>, cached: M::Result, lock_enabled: bool) -> Result<M::Result, CacheError> 
    where
        A: Actor + Handler<M>,
        <A as Actor>::Context: ToEnvelope<A, M>,
        S: Actor + Backend,
        <S as Actor>::Context:
            ToEnvelope<S, Get> + ToEnvelope<S, Set> + ToEnvelope<S, Lock> + ToEnvelope<S, Delete>,
        M: Message + Send + Cacheable,
        M::Result: MessageResponse<A, M> + CacheableResponse + Send,
        <<M as actix::Message>::Result as CacheableResponse>::Cached: DeserializeOwned,
    {
        #[cfg(feature = "metrics")]
        let (actor, message) = (msg.upstream_name(), msg.message.cache_key_prefix());
        #[cfg(feature = "metrics")]
        CACHE_STALE_COUNTER
            .with_label_values(&[&message, actor])
            .inc();
        let cache_key = msg.cache_key()?;
        if lock_enabled {
            let lock_status = backend
                .send(Lock {
                    key: "lock::test".to_owned(),
                    ttl: 10,
                })
                .await
                .unwrap_or_else(|error| {
                    warn!("Backend actor lock error {}", error);
                    Ok(LockStatus::Locked)
                })
                .unwrap_or_else(|error| {
                    warn!("Lock status retrieve error {}", error);
                    LockStatus::Locked
                });
            match lock_status {
                LockStatus::Acquired => {
                    debug!("Lock {} acquired", "HACK!");
                    let upstream_result = msg.upstream.send(msg.message).await?;
                    let result = match upstream_result.into_policy() {
                        CachePolicy::Cacheable(cached) => {
                            let cached = CachedValue::new(cached, 10);
                            cached
                                .store(backend, cache_key, Some(10))
                                .await
                                .unwrap_or_else(|error| {
                                    log::warn!("Updating cache error: {}", error);
                                });
                            Ok(CacheableResponse::from_cached(cached.into_inner()))
                        },
                        CachePolicy::NonCacheable(cached) => Ok(cached),
                    };
                    let _ = backend
                        .send(Delete { key: "lock::test".to_owned() })
                        .await
                        .map_err(|error| {
                            warn!("Lock error: {}", error);
                            error
                        });
                    result
                },
                LockStatus::Locked => {
                    Ok(cached)
                }
            }
        } else {
            Err(CacheError::CacheKeyGenerationError("Test".to_owned()))
        }
    }

    pub(crate) async fn handle_miss<A, S, M>(msg: QueryCache<A, M>, backend: &Addr<S>, cache_enabled: bool) -> Result<M::Result, CacheError>
    where
        A: Actor + Handler<M>,
        <A as Actor>::Context: ToEnvelope<A, M>,
        S: Actor + Backend,
        <S as Actor>::Context:
            ToEnvelope<S, Get> + ToEnvelope<S, Set> + ToEnvelope<S, Lock> + ToEnvelope<S, Delete>,
        M: Message + Send + Cacheable,
        M::Result: MessageResponse<A, M> + CacheableResponse + Send,
        <<M as actix::Message>::Result as CacheableResponse>::Cached: DeserializeOwned,
    {

        #[cfg(feature = "metrics")]
        let (actor, message) = (msg.upstream_name(), msg.message.cache_key_prefix());
        #[cfg(feature = "metrics")]
        CACHE_MISS_COUNTER
            .with_label_values(&[&message, actor])
            .inc();
        let cache_key = msg.cache_key()?;
        let cache_ttl = msg.message.cache_ttl();
        let cache_stale_ttl = msg.message.cache_stale_ttl();
        #[cfg(feature = "metrics")]
        let query_timer = CACHE_UPSTREAM_HANDLING_HISTOGRAM
            .with_label_values(&[&message, actor])
            .start_timer();
        let upstream_result = msg.upstream.send(msg.message).await?;
        #[cfg(feature = "metrics")]
        query_timer.observe_duration();
        if !cache_enabled {
            return Ok(upstream_result);
        }
        match upstream_result.into_policy() {
            CachePolicy::Cacheable(cached) => {
                let cached = CachedValue::new(cached, cache_stale_ttl);
                cached
                    .store(backend, cache_key, Some(cache_ttl))
                    .await
                    .unwrap_or_else(|error| {
                        log::warn!("Updating cache error: {}", error);
                    });
                Ok(CacheableResponse::from_cached(cached.into_inner()))
            },
            CachePolicy::NonCacheable(cached) => Ok(cached),
        }
    }


}

impl<B> Actor for CacheActor<B>
where
    B: Backend,
{
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
        debug!("Cache enabled: {}", self.enabled);
    }
}

/// Cache actor configurator.
///
/// # Example
/// ```rust
/// use actix::prelude::*;
/// use actix_cache::{Cache, RedisBackend, CacheError};
///
/// #[actix_rt::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let backend = RedisBackend::new()
///         .await?
///         .start();
///     let cache = Cache::builder()
///         .enabled(true)
///         .build(backend)
///         .start();
///     Ok(())
/// }
/// ```
pub struct CacheBuilder<B>
where
    B: Backend + Actor,
{
    enabled: bool,
    stale_cache_policy: StalePolicy,
    cache_lock_policy: CacheLockPolicy,
    _p: PhantomData<B>,
}

impl<B> Default for CacheBuilder<B>
where
    B: Backend,
{
    fn default() -> Self {
        CacheBuilder {
            enabled: true,
            stale_cache_policy: StalePolicy::default(),
            cache_lock_policy: CacheLockPolicy::default(),
            _p: PhantomData::default(),
        }
    }
}

impl<B> CacheBuilder<B>
where
    B: Backend,
{
    /// Enable or disable interaction with cache backend. (Enabled by default)
    ///
    /// All messages sent to disabled Cache actor passed directly to an upstream actor.
    /// If metrics feature is enabled, metrics for these messages collected too.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Enable stale cache mechanics. (Disabled by default)
    ///
    /// If it's enable cache actor return stale data in case of upstream [actor error].
    ///
    /// [actor error]: https://docs.rs/actix/latest/actix/enum.MailboxError.html
    pub fn enable_stale_cache(mut self) -> Self {
        self.stale_cache_policy = StalePolicy::Enabled;
        self
    }

    /// Disable cache lock mechanics. (Enabled by default)
    ///
    /// Prevents multiple upstream requests for the same cache key in case of cache data is missing.
    /// If `cache_lock` enabled only the first request will produce an upstream request. 
    /// The remaining requests wait for a first upstream response and return updated data.
    /// If `stale_cache` is enabled the remaining requests don't wait for an upstream response 
    /// and return stale cache data if it exists.
    pub fn disable_cache_lock(mut self) -> Self {
        self.cache_lock_policy = CacheLockPolicy::Disabled;
        self
    }

    /// Instatiate new [Cache] instance with current configuration and passed backend.
    ///
    /// Backend is an [Addr] of actix [Actor] which implements [Backend] trait:
    ///
    /// [Cache]: actor/struct.Cache.html
    /// [Backend]: ../dev/trait.Backend.html
    /// [Addr]: https://docs.rs/actix/latest/actix/prelude/struct.Addr.html
    /// [Actor]: https://docs.rs/actix/latest/actix/prelude/trait.Actor.html
    /// [Messages]: https://docs.rs/actix/latest/actix/prelude/trait.Message.html
    /// [Handler]: https://docs.rs/actix/latest/actix/prelude/trait.Handler.html
    pub fn build(self, backend: Addr<B>) -> CacheActor<B> {
        CacheActor {
            enabled: self.enabled,
            stale_cache_policy: self.stale_cache_policy,
            cache_lock_policy: self.cache_lock_policy,
            backend,
        }
    }
}
