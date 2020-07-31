#![warn(missing_docs)]
pub mod actor;
pub mod error;

#[doc(inline)]
pub use crate::actor::{RedisBackend, RedisBackendBuilder};
