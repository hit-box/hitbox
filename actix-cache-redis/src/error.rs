//! Error decplaration and transformation into [BackendError].
//!
//! [BackendError]: ../../actix_cache_backend/enum.BackendError.html
use actix_cache_backend::BackendError;
use redis::RedisError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
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
