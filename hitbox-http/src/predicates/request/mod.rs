//! Request predicates for cache eligibility.
//!
//! These predicates evaluate incoming HTTP requests to determine if a cache
//! lookup should be attempted.
//!
//! # Examples
//!
//! Cache only GET and HEAD requests:
//!
//! ```
//! use hitbox_http::predicates::request::{self, MethodPredicate};
//! use hitbox_http::predicates::request::method;
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! // Single method
//! let predicate = request::predicate::<Empty<Bytes>>()
//!     .method(method::Operation::eq(http::Method::GET));
//!
//! // Multiple methods
//! let predicate = request::predicate::<Empty<Bytes>>()
//!     .method(method::Operation::any(vec![
//!         http::Method::GET,
//!         http::Method::HEAD,
//!     ]));
//! ```
//!
//! Skip cache for requests with `Cache-Control: no-cache`:
//!
//! ```
//! use hitbox::predicate::PredicateExt;
//! use hitbox_http::predicates::header::{HeaderPredicate, Operation};
//! use hitbox_http::predicates::request;
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! let predicate = request::predicate::<Empty<Bytes>>()
//!     .header(Operation::Contains(
//!         http::header::CACHE_CONTROL,
//!         "no-cache".to_string(),
//!     ));
//! let predicate = predicate.not();
//! ```

pub mod body;
pub mod header;
/// HTTP method predicates for cache eligibility.
pub mod method;
pub mod path;
pub mod query;

pub use body::{Body, BodyPredicate};
pub use header::{Header, HeaderPredicate};
pub use method::{Method, MethodPredicate};
pub use path::{Path, PathPredicate};
pub use query::{Query, QueryPredicate};

use super::NeutralRequestPredicate;

/// Creates a neutral request predicate as the starting point for a predicate chain.
///
/// # Examples
///
/// ```ignore
/// use hitbox_http::predicates::request::{self, method, path, MethodPredicate, PathPredicate};
///
/// let pred = request::predicate()
///     .method(method::Operation::eq(http::Method::GET))
///     .path(path::Operation::pattern("/api/users/{id}"));
/// ```
pub fn predicate<ReqBody: http_body::Body>() -> NeutralRequestPredicate<ReqBody> {
    NeutralRequestPredicate::new()
}
