//! Error decplaration and transformation into [BackendError].
//!
//! [BackendError]: ../../hitbox_backend/enum.BackendError.html
use hitbox_backend::BackendError;
use redis::RedisError;

/// Redis backend error declaration.
///
/// Simply, it's just a wrapper for [redis::RedisError].
///
/// [redis::RedisError]: TODO
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Wrapper for all kinds redis-rs errors.
    #[error("Redis backend error: {0}")]
    Redis(RedisError),
}

impl From<RedisError> for Error {
    fn from(error: RedisError) -> Self {
        Error::Redis(error)
    }
}

impl From<Error> for BackendError {
    fn from(error: Error) -> Self {
        Self::InternalError(Box::new(error))
    }
}
