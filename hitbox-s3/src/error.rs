//! Error types for the S3 backend.

use aws_sdk_s3::error::SdkError;
use hitbox_backend::BackendError;
use thiserror::Error;

/// Errors that can occur when using [`S3Backend`](crate::S3Backend).
#[derive(Debug, Error)]
pub enum S3Error {
    /// An error returned by the S3 API (any operation).
    #[error("S3 operation failed: {0}")]
    S3(String),

    /// Failed to read the body of an S3 object after a successful response.
    #[error("failed to read S3 object body: {0}")]
    BodyRead(String),

    /// The provided backend configuration is invalid.
    #[error("invalid S3 backend configuration: {0}")]
    InvalidConfig(String),
}

/// Converts any AWS SDK error (`SdkError<E, R>`) into an [`S3Error::S3`].
///
/// A single blanket impl covers every operation error type (`GetObjectError`,
/// `PutObjectError`, `DeleteObjectError`, `CreateBucketError`, ...) without
/// enumerating each one.
impl<E, R> From<SdkError<E, R>> for S3Error
where
    SdkError<E, R>: std::fmt::Display,
{
    fn from(err: SdkError<E, R>) -> Self {
        S3Error::S3(err.to_string())
    }
}

impl From<S3Error> for BackendError {
    fn from(err: S3Error) -> Self {
        BackendError::InternalError(Box::new(err))
    }
}
