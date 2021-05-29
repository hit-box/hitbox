#![warn(missing_docs)]
//! hitbox [Backend] implementation for Redis.
//!
//! This crate uses [redis-rs] as base library for asynchronous interaction with redis nodes.
//! It use one [MultiplexedConnection] for maximum connection utilisation.
//!
//! [MultiplexedConnection]: redis::aio::MultiplexedConnection
//! [Backend]: hitbox_backend::Backend
//! [redis-rs]: redis-rs::aio
pub mod actor;
pub mod error;

#[doc(inline)]
pub use crate::actor::{RedisBackend, RedisBackendBuilder};
