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
//! Start with [`extractor()`] and chain extractors using Config types:
//!
//! ```
//! use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathExtractor};
//! use hitbox_http::extractors::query::QueryExtractor;
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox_http::extractors::{NeutralExtractor, Method, Path, query::Query};
//! let extractor = extractors::extractor::<Empty<Bytes>>()
//!     .method(MethodConfig::new())
//!     .path("/users/{user_id}/posts/{post_id}")
//!     .query("page")
//!     .query("limit");
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
//! - `Hash`: Full SHA256 hash (64 hex chars)
//! - `Lowercase`: Convert to lowercase
//! - `Uppercase`: Convert to uppercase
//! - `Truncate(n)`: Truncate to `n` characters
//!
//! [`Extractor`]: hitbox::Extractor
//! [`KeyPart`]: hitbox::KeyPart

use std::marker::PhantomData;

use async_trait::async_trait;
use hitbox::EvalContext;
use hitbox::{Extractor, KeyParts};

use crate::CacheableHttpRequest;

pub use method::{Method, MethodConfig, MethodExtractor};
pub use path::{Path, PathConfig, PathExtractor};
pub use version::{Version, VersionConfig, VersionExtractor};

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
/// start extractor chains with [`extractor()`] instead.
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
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Method, Path};
/// // The full type is Path<Method<NeutralExtractor<Empty<Bytes>>>>
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .path("/users/{id}");
/// # let _: &Path<Method<NeutralExtractor<Empty<Bytes>>>> = &extractor;
/// ```
#[derive(Debug)]
pub struct NeutralExtractor<ReqBody> {
    _res: PhantomData<fn(ReqBody) -> ReqBody>,
}

impl<ResBody> Default for NeutralExtractor<ResBody> {
    fn default() -> Self {
        NeutralExtractor { _res: PhantomData }
    }
}

impl<ResBody> NeutralExtractor<ResBody> {
    /// Creates a new neutral extractor.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl<ResBody> Extractor for NeutralExtractor<ResBody>
where
    ResBody: hyper::body::Body + Send + 'static,
    ResBody::Error: Send,
{
    type Subject = CacheableHttpRequest<ResBody>;

    async fn get(&self, subject: Self::Subject, _ctx: &mut EvalContext) -> KeyParts<Self::Subject> {
        KeyParts::new(subject)
    }
}

/// Creates a neutral extractor as the starting point for an extractor chain.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathConfig, PathExtractor};
/// use hitbox_http::extractors::query::{QueryConfig, QueryExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Method, Path, query::Query};
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .path(PathConfig::pattern("/users/{user_id}"))
///     .query("page");
/// # let _: &Query<Path<Method<NeutralExtractor<Empty<Bytes>>>>> = &extractor;
/// ```
pub fn extractor<ReqBody: hyper::body::Body>() -> NeutralExtractor<ReqBody> {
    NeutralExtractor::new()
}
