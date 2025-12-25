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
//! use hitbox_http::predicates::response::{StatusCode, StatusClass};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox::Neutral;
//! # use hitbox_http::CacheableHttpResponse;
//! # type Subject = CacheableHttpResponse<Empty<Bytes>>;
//! // Match 2xx status codes
//! let predicate = StatusCode::new(http::StatusCode::OK);
//! # let _: &StatusCode<Neutral<Subject>> = &predicate;
//! // Or match the entire success class
//! let predicate = StatusCode::new_class(Neutral::new(), StatusClass::Success);
//! # let _: &StatusCode<Neutral<Subject>> = &predicate;
//! ```
//!
//! Cache responses with non-empty JSON arrays:
//!
//! ```
//! use hitbox_http::predicates::response::{Body, Operation, JqExpression, JqOperation};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox::Neutral;
//! # use hitbox_http::CacheableHttpResponse;
//! # type Subject = CacheableHttpResponse<Empty<Bytes>>;
//! let predicate = Body::new(Operation::Jq {
//!     filter: JqExpression::compile(".items | length > 0").unwrap(),
//!     operation: JqOperation::Eq(serde_json::Value::Bool(true)),
//! });
//! # let _: &Body<Neutral<Subject>> = &predicate;
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
