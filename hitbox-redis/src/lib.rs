// TODO: add docs
#![allow(missing_docs)]
//! hitbox [Backend] implementation for Redis.
pub mod error;
mod backend;
mod builder;

pub use backend::RedisBackend;
pub use builder::Builder;
pub use error::Error;

