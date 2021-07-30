#![warn(missing_docs)]
//! hitbox [Backend] implementation for Redis.
//!
//! This crate uses [redis-rs] as base library for asynchronous interaction with redis nodes.
//! It use one [MultiplexedConnection] for better connection utilisation.
//!
//! [MultiplexedConnection]: redis::aio::MultiplexedConnection
//! [Backend]: hitbox_backend::Backend
//! [redis-rs]: redis-rs::aio
pub mod actor;
pub mod error;

#[doc(inline)]
pub use crate::actor::RedisBackend;
#[cfg(feature = "cluster")]
pub use crate::actor::{RedisClusterBackend, RedisClusterBuilder};
#[cfg(feature = "single")]
pub use crate::actor::{RedisSingleBackend, RedisSingleBuilder};

/// Type alias with RedisBackend with connection to redis single instanse.
#[cfg(feature = "cluster")]
pub type RedisCluster = actor::RedisBackend<RedisClusterBackend>;
/// Type alias with RedisBackend with connection to redis cluster instanse.
#[cfg(feature = "single")]
pub type RedisSingle = actor::RedisBackend<RedisSingleBackend>;
