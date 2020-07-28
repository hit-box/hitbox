use actix::dev::ToEnvelope;
use actix::prelude::*;
use thiserror::Error;

pub trait Backend
where
    Self: Actor + Handler<Set> + Handler<Get> + Handler<Lock> + Handler<Delete>,
{
    type Actor: Actor<Context = <Self as Backend>::Context>
        + Handler<Set>
        + Handler<Get>
        + Handler<Lock>
        + Handler<Delete>;
    type Context: ActorContext
        + ToEnvelope<Self::Actor, Get>
        + ToEnvelope<Self::Actor, Set>
        + ToEnvelope<Self::Actor, Lock>
        + ToEnvelope<Self::Actor, Delete>;
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error(transparent)]
    InternalError(Box<dyn std::error::Error + Send>),
    #[error(transparent)]
    ConnectionError(Box<dyn std::error::Error + Send>),
}

/// Actix message requests cache backend value by key.
#[derive(Message, Debug, Clone, PartialEq)]
#[rtype(result = "Result<Option<String>, BackendError>")]
pub struct Get {
    pub key: String,
}

/// Actix message writes cache backend value by key.
#[derive(Message, Debug, Clone, PartialEq)]
#[rtype(result = "Result<String, BackendError>")]
pub struct Set {
    pub key: String,
    pub value: String,
    pub ttl: Option<u32>,
}

/// Status of deleting result.
#[derive(Debug, PartialEq)]
pub enum DeleteStatus {
    /// Record sucessfully deleted.
    Deleted(u32),
    /// Record already missing.
    Missing,
}

/// Actix message delete record in backend by key.
#[derive(Message, Debug, Clone, PartialEq)]
#[rtype(result = "Result<DeleteStatus, BackendError>")]
pub struct Delete {
    pub key: String,
}

/// Actix message creates lock in cache backend.
#[derive(Message, Debug, Clone, PartialEq)]
#[rtype(result = "Result<LockStatus, BackendError>")]
pub struct Lock {
    pub key: String,
    pub ttl: u32,
}

/// Enum for representing status of Lock object in backend.
#[derive(Debug, PartialEq)]
pub enum LockStatus {
    /// Lock sucsesfully created and acquired.
    Acquired,
    /// Lock object already acquired (locked).
    Locked,
}
