//! Error declaration and transformation into [BackendError].
//!
//! [BackendError]: hitbox_backend::BackendError
use hitbox_backend::BackendError;
use redis::RedisError;

/// Redis backend error declaration.
///
/// Simply, it's just a wrapper for [redis::RedisError].
///
/// [redis::RedisError]: redis::RedisError
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Wrapper for all kinds redis-rs errors.
    #[error("Redis backend error: {0}")]
    Redis(#[from] RedisError),
}

impl From<Error> for BackendError {
    fn from(error: Error) -> Self {
        Self::InternalError(Box::new(error))
    }
}
