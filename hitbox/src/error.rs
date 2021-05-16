//! Error implementation and transformations.
use thiserror::Error;

/// Base hitbox error.
#[derive(Error, Debug)]
pub enum CacheError {
    /// Error described all problems with cache backend interactions.
    #[error(transparent)]
    BackendError(#[from] hitbox_backend::BackendError),
    /// Wrapper for upstream actor mailbox or timeout errors.
    #[error("Actix upstream actor error")]
    UpstreamError(#[from] actix::MailboxError),
    /// Wrapper for cache data serialization problems.
    #[error("Cached data serialization error")]
    SerializeError(#[from] serde_json::Error),
    /// Wrapper for cache data deserialization problems.
    #[error("Cached data deserialization error")]
    DeserializeError,
    /// Wrapper error for problems with cache key generation.
    #[error("Cache key generation error")]
    CacheKeyGenerationError(String),
}
