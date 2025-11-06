use bincode::error::{DecodeError, EncodeError};
use feoxdb::FeoxError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeOxDbError {
    #[error("FeOxDB error: {0}")]
    FeOxDb(#[from] FeoxError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] EncodeError),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] DecodeError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
