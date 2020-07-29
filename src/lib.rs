pub mod actor;
pub mod cache;
pub mod dev;
pub mod error;
#[cfg(feature = "metrics")]
pub mod metrics;

pub use actor::{Cache, CacheBuilder};
pub use cache::{Cacheable, QueryCache};
pub use error::CacheError;

pub use actix_cache_redis::actor::RedisActor as RedisBackend;

#[cfg(feature = "derive")]
pub use serde_qs;
