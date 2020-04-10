use actix::prelude::*;

#[derive(Debug)]
pub enum BackendError {
    Default    
}

/// Actix message requests cache backend value by key.
#[derive(Message, Debug)]
#[rtype(result = "Result<Option<String>, BackendError>")]
pub struct Get {
    pub key: String,
}

/// Actix message writes cache backend value by key.
#[derive(Message, Debug, Clone)]
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
#[derive(Message, Debug)]
#[rtype(result = "Result<DeleteStatus, BackendError>")]
pub struct Delete {
    pub key: String,
}

/// Actix message creates lock in cache backend.
#[derive(Message, Debug, Clone)]
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
