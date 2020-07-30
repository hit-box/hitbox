//! Cache actor and Builder.
use std::marker::PhantomData;

use actix::prelude::*;
use actix_cache_backend::Backend;
use actix_cache_redis::actor::RedisActor;
use log::{debug, info};

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
/// use actix_cache::{Cache as CacheActor, RedisBackend, CacheError};
///
/// type Cache = CacheActor<RedisBackend>;
///
/// #[actix_rt::main]
/// async fn main() -> Result<(), CacheError> {
///     let cache = Cache::new().await?.start();
///     Ok(())
/// }
/// ```
///
/// [QueryCache]: ../cache/struct.QueryCache.html
pub struct Cache<B>
where
    B: Backend,
{
    pub(crate) enabled: bool,
    pub(crate) backend: Addr<B>,
}

impl<B> Cache<B>
where
    B: Backend,
{
    /// Initialize new Cache actor with default [RedisBackend].
    ///
    /// [RedisBackend]: ../../actix_cache_redis/actor/struct.RedisActor.html
    pub async fn new() -> Result<Cache<RedisActor>, CacheError> {
        let backend = RedisActor::new()
            .await
            .map_err(|err| CacheError::BackendError(err.into()))?
            .start();
        Ok(CacheBuilder::default().build(backend))
    }

    /// Creates new [CacheBuilder](struct.CacheBuilder.html) instance for Cache actor configuration.
    pub fn builder() -> CacheBuilder<B> {
        CacheBuilder::default()
    }
}

impl<B> Actor for Cache<B>
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
    _p: PhantomData<B>,
}

impl<B> Default for CacheBuilder<B>
where
    B: Backend,
{
    fn default() -> Self {
        CacheBuilder {
            enabled: true,
            _p: PhantomData::default(),
        }
    }
}

impl<B> CacheBuilder<B>
where
    B: Backend,
{
    /// Enable or disable interaction with cache backend.
    ///
    /// All messages sent to disabled Cache actor passed directly to an upstream actor.
    /// If metrics feature is enabled, metrics for these messages collected too.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
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
    pub fn build(self, backend: Addr<B>) -> Cache<B> {
        Cache {
            enabled: self.enabled,
            backend,
        }
    }
}
