//! Error implementation and transformations.
use thiserror::Error;

/// Base hitbox error.
#[derive(Error, Debug)]
pub enum CacheError {
    /// Error described all problems with cache backend interactions.
    #[error(transparent)]
    BackendError(#[from] hitbox_backend::BackendError),
    /// Wrapper for upstream errors.
    #[error("Upstream error: {0}")]
    UpstreamError(Box<dyn std::error::Error + Send>),
    /// Wrapper error for problems with cache key generation.
    #[error("Cache key generation error")]
    CacheKeyGenerationError(String),
}
