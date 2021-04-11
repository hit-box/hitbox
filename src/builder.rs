use actix_cache_backend::Backend;
use actix::{Actor, Addr};
use std::marker::PhantomData;
use crate::CacheActor;
use crate::settings::{CacheSettings, Status, InitialCacheSettings};

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
                stale: Status::Disabled,
                lock: Status::Enabled,
            },
            _p: PhantomData::default(),
        }
    }
}

impl<B> CacheBuilder<B>
where
    B: Backend,
{
    pub fn enable(mut self) -> Self {
        self.settings.cache = Status::Enabled;
        self
    }

    pub fn disable(mut self) -> Self {
        self.settings.cache = Status::Disabled;
        self
    }

    pub fn with_stale(mut self) -> Self {
        self.settings.stale = Status::Enabled;
        self
    }

    pub fn without_stale(mut self) -> Self {
        self.settings.stale = Status::Disabled;
        self
    }

    pub fn with_lock(mut self) -> Self {
        self.settings.lock = Status::Enabled;
        self
    }

    pub fn without_lock(mut self) -> Self {
        self.settings.lock = Status::Disabled;
        self
    }

    /// Instantiate new [Cache] instance with current configuration and passed backend.
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
        let settings = InitialCacheSettings::from(self.settings);
        CacheActor { settings, backend, }
    }
}
