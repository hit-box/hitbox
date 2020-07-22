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
    #[cfg(feature = "derive")]
    #[error("Cache key serialization error")]
    SerializeCacheKeyError(#[from] serde_qs::Error),
}
