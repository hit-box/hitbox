use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache backend error")]
    BackendError(#[from] actix_cache_backend::BackendError),
    #[error("Actix upstream actor error")]
    UpstreamError(#[from] actix::MailboxError),
    #[error("Cached data serialization error")]
    SerializeError(#[from] serde_json::Error),
    #[error("Cached data deserialization error")]
    DeserializeError,
}
