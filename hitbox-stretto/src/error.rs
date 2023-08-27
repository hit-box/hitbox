use hitbox_backend::BackendError;
use stretto::CacheError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Cache backend error: cannot insert value")]
    Insert,
    #[error("Cache backend error: {0}")]
    CacheError(#[from] CacheError),
}

impl From<Error> for BackendError {
    fn from(error: Error) -> Self {
        Self::InternalError(Box::new(error))
    }
}
