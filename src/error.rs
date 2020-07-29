//! Error implementation and convertations.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error(transparent)]
    BackendError(#[from] actix_cache_backend::BackendError),
    #[error("Actix upstream actor error")]
    UpstreamError(#[from] actix::MailboxError),
    #[error("Cached data serialization error")]
    SerializeError(#[from] serde_json::Error),
    #[error("Cached data deserialization error")]
    DeserializeError,
    #[error("Cache key generation error")]
    CacheKeyGenerationError(String),
}
