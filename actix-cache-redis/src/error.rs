use actix_cache_backend::BackendError;
use redis::RedisError;

#[derive(Debug)]
pub enum Error {
    Redis(RedisError),
}

impl From<RedisError> for Error {
    fn from(error: RedisError) -> Self {
        Error::Redis(error)
    }
}

impl From<Error> for BackendError {
    fn from(_error: Error) -> Self {
        Self::Default
    }
}
