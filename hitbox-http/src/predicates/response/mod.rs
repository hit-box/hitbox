//! Response predicates for cache storage decisions.
//!
//! These predicates evaluate HTTP responses to determine if they should be
//! stored in the cache.
//!
//! # Examples
//!
//! Cache only successful responses:
//!
//! ```
//! use hitbox_http::predicates::response::{self, StatusCodePredicate, StatusClass, status};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! // Match 200 OK
//! let predicate = response::predicate::<Empty<Bytes>>()
//!     .status(status::Operation::eq(http::StatusCode::OK));
//!
//! // Or match the entire success class
//! let predicate = response::predicate::<Empty<Bytes>>()
//!     .status(status::Operation::class(StatusClass::Success));
//! ```
//!
//! Cache responses with non-empty JSON arrays:
//!
//! ```
//! use hitbox_http::predicates::response::{self, BodyPredicate, Operation, JqOperation};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! let predicate = response::predicate::<Empty<Bytes>>()
//!     .body(Operation::jq(".items | length > 0", JqOperation::Eq(serde_json::Value::Bool(true))).unwrap());
//! ```

pub mod body;
pub mod header;
/// HTTP status code predicates for cache storage.
pub mod status;

pub use body::{Body, BodyPredicate, JqFilter};
pub use header::{Header, HeaderPredicate};
pub use status::{StatusClass, StatusCode, StatusCodePredicate};

// Re-export shared body types for convenience
pub use crate::predicates::body::{JqExpression, JqOperation, Operation, PlainOperation};

use super::NeutralResponsePredicate;

/// Creates a neutral response predicate as the starting point for a predicate chain.
///
/// # Examples
///
/// ```ignore
/// use hitbox_http::predicates::response::{self, status, StatusClass, StatusCodePredicate};
///
/// let pred = response::predicate()
///     .status(status::Operation::class(StatusClass::Success));
/// ```
pub fn predicate<ResBody: http_body::Body>() -> NeutralResponsePredicate<ResBody> {
    NeutralResponsePredicate::new()
}
