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
//! use hitbox_http::predicates::request::Method;
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox::Neutral;
//! # use hitbox_http::CacheableHttpRequest;
//! # type Subject = CacheableHttpRequest<Empty<Bytes>>;
//! // Single method
//! let predicate = Method::new(http::Method::GET).unwrap();
//! # let _: &Method<Neutral<Subject>> = &predicate;
//!
//! // Multiple methods
//! let predicate = Method::new_in(
//!     Neutral::new(),
//!     vec![http::Method::GET, http::Method::HEAD],
//! );
//! # let _: &Method<Neutral<Subject>> = &predicate;
//! ```
//!
//! Skip cache for requests with `Cache-Control: no-cache`:
//!
//! ```
//! use hitbox::predicate::PredicateExt;
//! use hitbox_http::predicates::header::{Header, Operation};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox::Neutral;
//! # use hitbox_http::CacheableHttpRequest;
//! # type Subject = CacheableHttpRequest<Empty<Bytes>>;
//! let predicate = Header::new(Operation::Contains(
//!     http::header::CACHE_CONTROL,
//!     "no-cache".to_string(),
//! ));
//! # let _: &Header<Neutral<Subject>> = &predicate;
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
