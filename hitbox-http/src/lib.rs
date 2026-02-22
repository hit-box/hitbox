#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

pub mod body;
mod cache_status;
mod cacheable;
pub mod extractors;
pub mod predicates;
pub mod query;
/// HTTP request types and cache policy evaluation.
///
/// Contains [`CacheableHttpRequest`] — the core wrapper that pairs request
/// metadata with a [`BufferedBody`] for predicate/extractor inspection.
///
/// Also re-exports [`predicate()`] and [`extractor()`] entry points for
/// building request-side predicate and extractor chains.
pub mod request;
/// HTTP response types, cache storage, and serialization.
///
/// Contains [`CacheableHttpResponse`] for cache policy evaluation and
/// [`SerializableHttpResponse`] — the serialized form stored in cache backends.
///
/// Also re-exports [`predicate()`] entry point for building response predicate chains.
pub mod response;

pub use body::{BufferedBody, CollectExactResult, PartialBufferedBody, Remaining};
pub use cache_status::DEFAULT_CACHE_STATUS_HEADER;
pub use cacheable::CacheableSubject;
pub use request::CacheableHttpRequest;
pub use response::{CacheableHttpResponse, SerializableHttpResponse};
