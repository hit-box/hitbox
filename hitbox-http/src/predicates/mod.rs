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
//! use hitbox_http::predicates::request::{self, MethodPredicate, method};
//! use hitbox_http::predicates::header::{HeaderPredicate, Operation};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! // Cache GET requests without Cache-Control: no-cache
//! let predicate = request::predicate::<Empty<Bytes>>()
//!     .method(method::Operation::eq(http::Method::GET));
//! let predicate = predicate.and(
//!     request::predicate::<Empty<Bytes>>()
//!         .header(Operation::contains(http::header::CACHE_CONTROL, "no-cache"))
//!         .not()
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

/// A neutral predicate for HTTP requests that always returns `Cacheable`.
///
/// Use this when you want to cache all requests regardless of their properties,
/// or as a starting point for predicate chains with `PredicateExt`.
pub type NeutralRequestPredicate<ReqBody> = Neutral<CacheableHttpRequest<ReqBody>>;

/// A neutral predicate for HTTP responses that always returns `Cacheable`.
///
/// Use this when you want to cache all responses regardless of their properties,
/// or as a starting point for predicate chains with `PredicateExt`.
pub type NeutralResponsePredicate<ResBody> = Neutral<CacheableHttpResponse<ResBody>>;
