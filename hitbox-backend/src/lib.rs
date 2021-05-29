#![warn(missing_docs)]
//! Traits and struct messages for hitbox backend interaction.
//!
//! If you want implement your own backend, you in the right place.
use actix::dev::ToEnvelope;
use actix::prelude::*;
use thiserror::Error;

/// Define the behavior needed of an cache layer to work with cache backend.
///
/// Ultimately the implementing type must be an Actix `Actor` and it must implement handlers for a
/// specific set of message types:
///
/// * [Get]
/// * [Set]
/// * [Lock]
/// * [Delete]
///
/// [Get]: crate::Get
/// [Set]: crate::Set
/// [Delete]: crate::Delete
/// [Lock]: crate::Lock
pub trait Backend
where
    Self: Actor + Handler<Set> + Handler<Get> + Handler<Lock> + Handler<Delete>,
{
    /// Type of backend actor bound.
    type Actor: Actor<Context = <Self as Backend>::Context>
        + Handler<Set>
        + Handler<Get>
        + Handler<Lock>
        + Handler<Delete>;
    /// Type for backend Actor context.
    type Context: ActorContext
        + ToEnvelope<Self::Actor, Get>
        + ToEnvelope<Self::Actor, Set>
        + ToEnvelope<Self::Actor, Lock>
        + ToEnvelope<Self::Actor, Delete>;
}

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
}

/// Actix message requests cache backend value by key.
#[derive(Message, Debug, Clone, PartialEq)]
#[rtype(result = "Result<Option<Vec<u8>>, BackendError>")]
pub struct Get {
    /// Key of cache backend record.
    pub key: String,
}

/// Actix message writes cache backend value by key.
#[derive(Message, Debug, Clone, PartialEq)]
#[rtype(result = "Result<String, BackendError>")]
pub struct Set {
    /// Key of cache backend record.
    pub key: String,
    /// Data for sorage by cache key.
    pub value: Vec<u8>,
    /// Optional value of time-to-live for cache record.
    pub ttl: Option<u32>,
}

/// Status of deleting result.
#[derive(Debug, PartialEq)]
pub enum DeleteStatus {
    /// Record successfully deleted.
    Deleted(u32),
    /// Record already missing.
    Missing,
}

/// Actix message delete record in backend by key.
#[derive(Message, Debug, Clone, PartialEq)]
#[rtype(result = "Result<DeleteStatus, BackendError>")]
pub struct Delete {
    /// Key of cache backend record for deleting
    pub key: String,
}

/// Actix message creates lock in cache backend.
#[derive(Message, Debug, Clone, PartialEq)]
#[rtype(result = "Result<LockStatus, BackendError>")]
pub struct Lock {
    /// Key of cache backend record for lock.
    pub key: String,
    /// Time-to-live for cache key lock record.
    pub ttl: u32,
}

/// Enum for representing status of Lock object in backend.
#[derive(Debug, PartialEq)]
pub enum LockStatus {
    /// Lock successfully created and acquired.
    Acquired,
    /// Lock object already acquired (locked).
    Locked,
}
