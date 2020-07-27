pub mod actor;
pub mod dev;
pub mod cache;
pub mod error;

pub use error::CacheError;
pub use actor::{Cache, CacheBuilder};
pub use cache::{QueryCache, Cacheable};

pub use actix_cache_redis::actor::RedisActor as RedisBackend;

#[cfg(feature = "derive")]
pub use serde_qs;
