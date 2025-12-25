//! Predicates for determining HTTP request and response cacheability.
//!
//! Predicates evaluate HTTP messages and return [`Cacheable`] or [`NonCacheable`].
//! They can be combined using logical operators from the [`conditions`] module.
//!
//! # Request vs Response Predicates
//!
//! - **Request predicates** decide whether to attempt a cache lookup
//! - **Response predicates** decide whether to store a response in cache
//!
//! # Available Predicates
//!
//! ## Request Predicates ([`request`] module)
//!
//! | Predicate | Description |
//! |-----------|-------------|
//! | [`request::Method`] | Match by HTTP method |
//! | [`request::Path`] | Match by URL path pattern |
//! | [`request::Header`] | Match by request header |
//! | [`request::Query`] | Match by query parameter |
//! | [`request::Body`] | Match by request body content |
//!
//! ## Response Predicates ([`response`] module)
//!
//! | Predicate | Description |
//! |-----------|-------------|
//! | [`response::StatusCode`] | Match by status code or class |
//! | [`response::Header`] | Match by response header |
//! | [`response::Body`] | Match by response body content |
//!
//! # Combining Predicates
//!
//! Use [`PredicateExt`] to combine predicates:
//!
//! ```
//! use hitbox::predicate::PredicateExt;
//! use hitbox_http::predicates::request::Method;
//! use hitbox_http::predicates::header::{Header, Operation};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox::Neutral;
//! # use hitbox::predicate::And;
//! # use hitbox_http::CacheableHttpRequest;
//! # type Subject = CacheableHttpRequest<Empty<Bytes>>;
//! // Cache GET requests without Cache-Control: no-cache
//! let predicate = Method::new(http::Method::GET).unwrap();
//! # let _: &Method<Neutral<Subject>> = &predicate;
//! let predicate = predicate.and(
//!     Header::new(Operation::Contains(
//!         http::header::CACHE_CONTROL,
//!         "no-cache".to_string(),
//!     )).not()
//! );
//! ```
//!
//! [`Cacheable`]: hitbox::predicate::PredicateResult::Cacheable
//! [`NonCacheable`]: hitbox::predicate::PredicateResult::NonCacheable
//! [`PredicateExt`]: conditions::PredicateExt

use hitbox::Neutral;

use crate::{CacheableHttpRequest, CacheableHttpResponse};

pub mod body;
pub mod conditions;
pub mod header;
pub mod request;
pub mod response;
pub mod version;

/// Internal type: a neutral predicate for HTTP requests that always returns `Cacheable`.
///
/// Used internally by [`HttpEndpoint`](crate::HttpEndpoint). Users should start
/// predicate chains with concrete types like [`request::Method`] or [`request::Header`].
pub type NeutralRequestPredicate<ReqBody> = Neutral<CacheableHttpRequest<ReqBody>>;

/// Internal type: a neutral predicate for HTTP responses that always returns `Cacheable`.
///
/// Used internally by [`HttpEndpoint`](crate::HttpEndpoint). Users should start
/// predicate chains with concrete types like [`response::StatusCode`] or [`response::Header`].
pub type NeutralResponsePredicate<ResBody> = Neutral<CacheableHttpResponse<ResBody>>;
