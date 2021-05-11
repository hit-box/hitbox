//! Cache actor and Builder.
use crate::builder::CacheBuilder;
use actix::dev::ToEnvelope;
use actix::prelude::*;
use hitbox::dev::{Delete, Get, Lock, Set};
#[cfg(feature = "metrics")]
use hitbox::metrics::{
    CACHE_HIT_COUNTER, CACHE_MISS_COUNTER, CACHE_STALE_COUNTER, CACHE_UPSTREAM_HANDLING_HISTOGRAM,
};
use hitbox::settings::InitialCacheSettings;
use hitbox::CacheError;
use hitbox_backend::Backend;
use hitbox_redis::RedisBackend;
use tracing::{debug, info};

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
/// use hitbox_actix::{Cache, RedisBackend, CacheError};
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
    pub settings: InitialCacheSettings,
    pub(crate) backend: Addr<B>,
}

impl<B> CacheActor<B>
where
    B: Actor + Backend,
    <B as Actor>::Context:
        ToEnvelope<B, Get> + ToEnvelope<B, Set> + ToEnvelope<B, Lock> + ToEnvelope<B, Delete>,
{
    /// Initialize new Cache actor with default [RedisBackend].
    ///
    /// [RedisBackend]: ../../hitbox_redis/actor/struct.RedisActor.html
    pub async fn new() -> Result<CacheActor<RedisBackend>, CacheError> {
        let backend = RedisBackend::new()
            .await
            .map_err(|err| CacheError::BackendError(err.into()))?
            .start();
        Ok(CacheBuilder::default().finish(backend))
    }

    /// Creates new [CacheBuilder](struct.CacheBuilder.html) instance for Cache actor configuration.
    pub fn builder() -> CacheBuilder<B> {
        CacheBuilder::default()
    }
}

impl<B> Actor for CacheActor<B>
where
    B: Backend,
{
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
        debug!("Cache enabled: {:?}", self.settings);
    }
}
