// #![warn(missing_docs)]
//! Traits and structs for hitbox backend interaction.
//!
//! If you want implement your own backend, you in the right place.
mod backend;
pub mod predicates;
mod response;
pub mod serializer;
mod value;

pub use backend::{BackendResult, CacheBackend};
pub use response::{CachePolicy, CacheState, CacheableResponse};
use serializer::SerializerError;
use thiserror::Error;
pub use value::{CachedValue, EvictionPolicy, TtlSettings};

/// Proxy Error describes general groups of errors in backend interaction process.
#[derive(Debug, Error)]
pub enum BackendError {
    /// Internal backend error, state or computation error.
    ///
    /// Any error not bounded with network interaction.
    #[error(transparent)]
    InternalError(Box<dyn std::error::Error + Send>),
    /// Network interaction error.
    #[error(transparent)]
    ConnectionError(Box<dyn std::error::Error + Send>),
    /// Serializing\Deserializing data error.
    #[error(transparent)]
    SerializerError(#[from] SerializerError),
}

/// Status of deleting result.
#[derive(Debug, PartialEq, Eq)]
pub enum DeleteStatus {
    /// Record successfully deleted.
    Deleted(u32),
    /// Record already missing.
    Missing,
}

/// Enum for representing status of Lock object in backend.
#[derive(Debug, PartialEq, Eq)]
pub enum LockStatus {
    /// Lock successfully created and acquired.
    Acquired,
    /// Lock object already acquired (locked).
    Locked,
}
