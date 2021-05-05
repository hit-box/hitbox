//! Cache actor and Builder.
use actix::prelude::*;
use hitbox_backend::Backend;
use hitbox_redis::RedisBackend;
use log::{debug, info};

#[cfg(feature = "metrics")]
use crate::metrics::{
    CACHE_HIT_COUNTER, CACHE_MISS_COUNTER, CACHE_STALE_COUNTER, CACHE_UPSTREAM_HANDLING_HISTOGRAM,
};
use crate::CacheError;

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
/// use hitbox::{Cache, RedisBackend, CacheError};
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

use crate::builder::CacheBuilder;
use crate::settings::InitialCacheSettings;
use crate::{
    dev::{Delete, Get, Lock, Set},
};
use actix::dev::{MessageResponse, ToEnvelope};
use serde::de::DeserializeOwned;

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
