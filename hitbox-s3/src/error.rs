//! Error types for the S3 backend.

use aws_sdk_s3::error::SdkError;
use hitbox_backend::BackendError;
use thiserror::Error;

/// Errors that can occur when using [`S3Backend`](crate::S3Backend).
#[derive(Debug, Error)]
pub enum S3Error {
    /// An error returned by the S3 API (any operation).
    ///
    /// The original AWS SDK error is preserved as the [`source`](std::error::Error::source)
    /// so callers can downcast to inspect it (e.g. to distinguish throttling or
    /// a 5xx from a fatal error) and walk the full error chain.
    #[error("S3 operation failed: {0}")]
    S3(#[source] Box<dyn std::error::Error + Send + Sync>),

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
/// enumerating each one. The SDK error is boxed as the variant's `source`, so
/// the typed error and its chain survive the conversion.
impl<E, R> From<SdkError<E, R>> for S3Error
where
    SdkError<E, R>: std::error::Error + Send + Sync + 'static,
{
    fn from(err: SdkError<E, R>) -> Self {
        S3Error::S3(Box::new(err))
    }
}

impl From<S3Error> for BackendError {
    fn from(err: S3Error) -> Self {
        BackendError::InternalError(Box::new(err))
    }
}
