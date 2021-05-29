#![warn(missing_docs)]

pub mod actor;
pub mod builder;
pub mod handlers;
pub mod messages;
pub mod runtime;

pub use actor::CacheActor;
pub use builder::CacheBuilder;
pub use hitbox::{CacheError, Cacheable};
pub use messages::{IntoCache, QueryCache};
pub use runtime::ActixAdapter;

#[cfg(feature = "redis")]
pub use hitbox_redis::RedisBackend;

/// Default type alias with RedisBackend.
/// You can disable it or define it manually in your code.
#[cfg(feature = "redis")]
pub type Cache = CacheActor<RedisBackend>;

/// Prelude for hitbox_actix.
pub mod prelude {
    #[cfg(feature = "redis")]
    pub use crate::{Cache, RedisBackend};
    pub use crate::{CacheActor, CacheBuilder, CacheError, Cacheable, IntoCache, QueryCache};
    pub use hitbox::hitbox_serializer;
}
