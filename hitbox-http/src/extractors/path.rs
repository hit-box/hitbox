use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use crate::CacheableHttpRequest;

/// Extracts path parameters as cache key parts.
///
/// Uses [actix-router](https://docs.rs/actix-router) patterns to match and
/// extract named segments from the request path.
///
/// # Type Parameters
///
/// * `E` - The inner extractor to chain with. Use [`extractors::extractor()`](super::extractor)
///   to start a new chain, then call `.path(...)`.
///
/// # Pattern Syntax
///
/// - `{name}` — captures a path segment (characters until `/`)
/// - `{name:regex}` — captures with regex constraint (e.g., `{id:\d+}`)
/// - `{tail}*` — captures remaining path (e.g., `/blob/{path}*` matches `/blob/a/b/c`)
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Method, Path};
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .path("/users/{user_id}/posts/{post_id}");
/// # let _: &Path<Method<NeutralExtractor<Empty<Bytes>>>> = &extractor;
/// ```
///
/// # Key Parts Generated
///
/// For path `/users/42/posts/123` with pattern `/users/{user_id}/posts/{post_id}`:
/// - `KeyPart { key: "user_id", value: Some("42") }`
/// - `KeyPart { key: "post_id", value: Some("123") }`
///
/// # Format Examples
///
/// | Request Path | Pattern | Generated Key Parts |
/// |--------------|---------|---------------------|
/// | `/users/42` | `/users/{id}` | `id=42` |
/// | `/api/v2/items` | `/api/{version}/items` | `version=v2` |
/// | `/files/docs/report.pdf` | `/files/{path}*` | `path=docs/report.pdf` |
/// | `/orders/123/items/456` | `/orders/{order_id}/items/{item_id}` | `order_id=123&item_id=456` |
#[derive(Debug)]
pub struct Path<E> {
    inner: E,
    resource: ResourceDef,
}

/// Configuration for the path extractor.
///
/// Wraps a path pattern string using [actix-router](https://docs.rs/actix-router) syntax.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, PathConfig, PathExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .path(PathConfig::pattern("/users/{user_id}/posts/{post_id}"));
/// ```
#[derive(Debug, Clone)]
pub struct PathConfig {
    pub(crate) pattern: String,
}

impl PathConfig {
    /// Creates a path extractor configuration with the given pattern.
    ///
    /// See [`Path`] for pattern syntax documentation.
    pub fn pattern(pattern: impl Into<String>) -> Self {
        PathConfig {
            pattern: pattern.into(),
        }
    }
}

impl From<&str> for PathConfig {
    fn from(pattern: &str) -> Self {
        PathConfig::pattern(pattern)
    }
}

impl From<String> for PathConfig {
    fn from(pattern: String) -> Self {
        PathConfig::pattern(pattern)
    }
}

impl From<&String> for PathConfig {
    fn from(pattern: &String) -> Self {
        PathConfig::pattern(pattern.as_str())
    }
}

/// Extension trait for adding path extraction to an extractor chain.
///
/// # For Callers
///
/// Chain this to extract named segments from the request path. Each captured
/// segment becomes a cache key part. Use patterns like `/users/{user_id}` to
/// capture dynamic path segments.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Extractor`]
/// types. You don't need to implement it manually.
pub trait PathExtractor: Sized {
    /// Adds path parameter extraction with the given configuration.
    ///
    /// Accepts a [`PathConfig`] or a string pattern directly.
    ///
    /// See [`Path`] for pattern syntax documentation.
    fn path(self, config: impl Into<PathConfig>) -> Path<Self>;
}

impl<E> PathExtractor for E
where
    E: Extractor,
{
    fn path(self, config: impl Into<PathConfig>) -> Path<Self> {
        let config = config.into();
        Path {
            inner: self,
            resource: ResourceDef::from(config.pattern.as_str()),
        }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Path<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;
    type Context = E::Context;

    async fn get(
        &self,
        subject: Self::Subject,
        ctx: &mut Self::Context,
    ) -> KeyParts<Self::Subject> {
        let mut path = actix_router::Path::new(subject.parts().uri.path());
        self.resource.capture_match_info(&mut path);
        let mut matched_parts = path
            .iter()
            .map(|(key, value)| KeyPart::new(key, Some(value)))
            .collect::<Vec<_>>();
        let mut parts = self.inner.get(subject, ctx).await;
        parts.append(&mut matched_parts);
        parts
    }
}
