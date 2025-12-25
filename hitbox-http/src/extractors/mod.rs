//! Cache key extractors for HTTP requests.
//!
//! Extractors generate cache key parts from HTTP request components. They
//! implement the [`Extractor`] trait and can be chained using the builder pattern.
//!
//! # Available Extractors
//!
//! | Extractor | Description |
//! |-----------|-------------|
//! | [`Method`] | Extract HTTP method (GET, POST, etc.) |
//! | [`Path`] | Extract path parameters using patterns like `/users/{id}` |
//! | [`header::Header`] | Extract header values |
//! | [`query::Query`] | Extract query parameters |
//! | [`body::Body`] | Extract from body (hash, JQ, regex) |
//! | [`Version`] | Extract HTTP version |
//!
//! # Builder Pattern
//!
//! Start with [`Method::new()`] and chain other extractors:
//!
//! ```
//! use hitbox_http::extractors::{Method, path::PathExtractor, query::QueryExtractor};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox_http::extractors::{NeutralExtractor, Path, query::Query};
//! let extractor = Method::new()
//!     .path("/users/{user_id}/posts/{post_id}")
//!     .query("page".to_string())
//!     .query("limit".to_string());
//! # let _: &Query<Query<Path<Method<NeutralExtractor<Empty<Bytes>>>>>> = &extractor;
//! ```
//!
//! # Cache Key Structure
//!
//! Each extractor adds [`KeyPart`]s to the cache key. A `KeyPart` has:
//! - A name (e.g., "user_id", "page", "method")
//! - An optional value (e.g., "42", "1", "GET")
//!
//! The final cache key is computed from all collected parts.
//!
//! # Transforms
//!
//! Header and query extractors support value transformations via [`transform::Transform`]:
//! - `Hash`: SHA256 hash (truncated to 16 hex chars)
//! - `Lowercase`: Convert to lowercase
//! - `Uppercase`: Convert to uppercase
//!
//! [`Extractor`]: hitbox::Extractor
//! [`KeyPart`]: hitbox::KeyPart

use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use hitbox::{Extractor, KeyParts};

use crate::CacheableHttpRequest;

pub use method::Method;
pub use path::Path;
pub use version::Version;

pub mod body;
pub mod header;
/// HTTP method extraction for cache keys.
pub mod method;
/// Path parameter extraction for cache keys.
pub mod path;
pub mod query;
pub mod transform;
pub mod version;

/// Base extractor that produces an empty cache key.
///
/// This is an internal building block used by other extractors. Users should
/// start extractor chains with [`Method::new()`] instead.
///
/// # Type Parameters
///
/// * `ReqBody` - The HTTP request body type. Must implement [`hyper::body::Body`]
///   with `Send` bounds. This parameter propagates through extractor chains
///   to ensure type safety.
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It appears as the innermost type
/// in extractor chains:
///
/// ```
/// use hitbox_http::extractors::{Method, path::PathExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Path};
/// // The full type is Path<Method<NeutralExtractor<Empty<Bytes>>>>
/// let extractor = Method::new().path("/users/{id}");
/// # let _: &Path<Method<NeutralExtractor<Empty<Bytes>>>> = &extractor;
/// ```
#[derive(Debug)]
pub struct NeutralExtractor<ReqBody> {
    _res: PhantomData<fn(ReqBody) -> ReqBody>,
}

impl<ResBody> NeutralExtractor<ResBody> {
    /// Creates a new neutral extractor.
    pub fn new() -> Self {
        NeutralExtractor { _res: PhantomData }
    }
}

#[async_trait]
impl<ResBody> Extractor for NeutralExtractor<ResBody>
where
    ResBody: hyper::body::Body + Send + 'static + Debug,
    ResBody::Error: Send,
{
    type Subject = CacheableHttpRequest<ResBody>;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        KeyParts::new(subject)
    }
}

impl<ResBody> Default for NeutralExtractor<ResBody> {
    fn default() -> Self {
        Self::new()
    }
}
