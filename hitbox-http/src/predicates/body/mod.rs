//! Body content predicates for cache decisions.
//!
//! These predicates evaluate the body content of requests or responses.
//!
//! # Operations
//!
//! | Operation | Description |
//! |-----------|-------------|
//! | [`Operation::Limit`] | Cache only if body size is within limit |
//! | [`Operation::Plain`] | Match body as plain text (contains, regex, etc.) |
//! | [`Operation::Jq`] | Evaluate body as JSON using JQ expressions |
//!
//! # Caveats
//!
//! Body predicates consume bytes from the stream. After evaluation:
//! - The body transitions to [`BufferedBody::Partial`] or [`BufferedBody::Complete`]
//! - Subsequent predicates receive the modified body state
//! - Order your predicates to minimize body consumption
//!
//! # Examples
//!
//! Only cache responses with non-empty JSON arrays:
//!
//! ```
//! use hitbox_http::predicates::body::{Operation, JqExpression, JqOperation};
//!
//! let op = Operation::Jq {
//!     filter: JqExpression::compile(".items | length > 0").unwrap(),
//!     operation: JqOperation::Eq(serde_json::Value::Bool(true)),
//! };
//! ```
//!
//! [`BufferedBody::Partial`]: crate::BufferedBody::Partial
//! [`BufferedBody::Complete`]: crate::BufferedBody::Complete

mod jq;
mod operation;
mod plain;
mod predicate;

pub use jq::{JqExpression, JqOperation};
pub use operation::Operation;
pub use plain::PlainOperation;
pub use predicate::{Body, BodyPredicate};
