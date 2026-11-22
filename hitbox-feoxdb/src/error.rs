use feoxdb::FeoxError;
use thiserror::Error;

/// Errors that can occur when using [`FeOxDbBackend`](crate::FeOxDbBackend).
#[derive(Debug, Error)]
pub enum FeOxDbError {
    /// An error from the underlying FeOxDB database.
    #[error("FeOxDB error: {0}")]
    FeOxDb(#[from] FeoxError),

    /// An I/O error occurred while accessing the database file.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The provided configuration is invalid.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
