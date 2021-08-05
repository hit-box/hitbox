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

#[cfg(feature = "cluster")]
use crate::actor::RedisClusterBackend;
#[cfg(feature = "single")]
use crate::actor::RedisSingleBackend;
#[doc(inline)]
pub use crate::actor::{RedisBackend, RedisBuilder};

/// Type alias with RedisBackend with connection to redis single instanse.
#[cfg(feature = "cluster")]
pub type RedisCluster = actor::RedisBackend<RedisClusterBackend>;
/// Type alias with RedisBackend with connection to redis cluster instanse.
#[cfg(feature = "single")]
pub type RedisSingle = actor::RedisBackend<RedisSingleBackend>;
