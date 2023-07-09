//! Error declaration and transformation into [BackendError].
//!
//! [BackendError]: hitbox_backend::BackendError
use hitbox_backend::BackendError;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Redis backend error: {0}")]
    Redis(#[from] fred::error::RedisError),
    #[error("Builder error: {0}")]
    Builder(String),
    #[error(transparent)]
    Tokio(#[from] tokio::task::JoinError),
}

impl From<Error> for BackendError {
    fn from(error: Error) -> Self {
        Self::InternalError(Box::new(error))
    }
}
