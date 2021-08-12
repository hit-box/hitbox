//! CacheActor builder patter implementation.
use crate::CacheActor;
use actix::{Actor, Addr};
use hitbox::settings::{CacheSettings, Status};
use hitbox_backend::Backend;
use std::marker::PhantomData;

/// Cache actor configurator.
///
/// # Example
/// ```rust
/// use actix::prelude::*;
/// use hitbox_actix::{Cache, RedisBuilder, CacheError};
///
/// #[actix::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let backend = RedisBuilder::standalone("redis://127.0.0.1:6379")
///         .await?
///         .start();
///     let cache = Cache::builder()
///         .enable()
///         .finish(backend)
///         .start();
///     Ok(())
/// }
/// ```
pub struct CacheBuilder<B>
where
    B: Backend + Actor,
{
    settings: CacheSettings,
    _p: PhantomData<B>,
}

impl<B> Default for CacheBuilder<B>
where
    B: Backend,
{
    fn default() -> Self {
        CacheBuilder {
            settings: CacheSettings {
                cache: Status::Enabled,
                stale: Status::Enabled,
                lock: Status::Disabled,
            },
            _p: PhantomData::default(),
        }
    }
}

impl<B> CacheBuilder<B>
where
    B: Backend,
{
    /// Enable interaction with cache backend. (Default value).
    pub fn enable(mut self) -> Self {
        self.settings.cache = Status::Enabled;
        self
    }

    /// Disable interaction with cache backend.
    ///
    /// All messages sent to disabled Cache actor passed directly to an upstream actor.
    pub fn disable(mut self) -> Self {
        self.settings.cache = Status::Disabled;
        self
    }

    /// Enable stale cache mechanics. (Default value).
    ///
    /// If [CacheActor] receives a stale value, it does not return it immediately.
    /// It polls data from upstream, and if the upstream returned an error,
    /// the [CacheActor] returns a stale value. If no error occurred in the upstream,
    /// then a fresh value is stored in the cache and returned.
    pub fn with_stale(mut self) -> Self {
        self.settings.stale = Status::Enabled;
        self
    }

    /// Disable stale cache mechanics.
    pub fn without_stale(mut self) -> Self {
        self.settings.stale = Status::Disabled;
        self
    }

    /// Enable cache lock mechanics.
    ///
    /// Prevents multiple upstream requests for the same cache key in case of cache data is missing.
    /// Only the first request will produce an upstream request.
    /// The remaining requests wait for a first upstream response and return updated data.
    /// If `with_stale` is enabled the remaining requests don't wait for an upstream response
    /// and return stale cache data if it exists.
    pub fn with_lock(mut self) -> Self {
        self.settings.lock = Status::Enabled;
        self
    }

    /// Disable cache lock mechanics. (Default value).
    pub fn without_lock(mut self) -> Self {
        self.settings.lock = Status::Disabled;
        self
    }

    /// Instantiate new [Cache] instance with current configuration and passed backend.
    ///
    /// Backend is an [Addr] of actix [Actor] which implements [Backend] trait:
    ///
    /// [Cache]: crate::Cache
    /// [Backend]: hitbox_backend::Backend
    /// [Addr]: https://docs.rs/actix/latest/actix/prelude/struct.Addr.html
    /// [Actor]: https://docs.rs/actix/latest/actix/prelude/trait.Actor.html
    /// [Messages]: https://docs.rs/actix/latest/actix/prelude/trait.Message.html
    /// [Handler]: https://docs.rs/actix/latest/actix/prelude/trait.Handler.html
    pub fn finish(self, backend: Addr<B>) -> CacheActor<B> {
        CacheActor {
            settings: self.settings,
            backend,
        }
    }
}
