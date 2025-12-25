use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use super::NeutralExtractor;
use crate::CacheableHttpRequest;

/// Extracts path parameters as cache key parts.
///
/// Uses [actix-router](https://docs.rs/actix-router) patterns to match and
/// extract named segments from the request path.
///
/// # Type Parameters
///
/// * `E` - The inner extractor to chain with. Use [`Path::new`] to start
///   a new extractor chain (uses [`NeutralExtractor`] internally), or use the
///   [`PathExtractor`] extension trait to chain onto an existing extractor.
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
/// use hitbox_http::extractors::Path;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::NeutralExtractor;
/// // Extract user_id and post_id from "/users/42/posts/123"
/// let extractor = Path::new("/users/{user_id}/posts/{post_id}");
/// # let _: &Path<NeutralExtractor<Empty<Bytes>>> = &extractor;
/// ```
///
/// Using the builder pattern:
///
/// ```
/// use hitbox_http::extractors::{Method, path::PathExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Path};
/// let extractor = Method::new()
///     .path("/api/v1/users/{user_id}");
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

impl<S> Path<NeutralExtractor<S>> {
    /// Creates a path extractor that captures named segments from request paths.
    ///
    /// Each captured segment becomes a cache key part with the segment name
    /// as key. See the struct documentation for pattern syntax.
    ///
    /// Chain onto existing extractors using [`PathExtractor::path`] instead
    /// if you already have an extractor chain.
    pub fn new(resource: &str) -> Self {
        Self {
            inner: NeutralExtractor::new(),
            resource: ResourceDef::from(resource),
        }
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
    /// Adds path parameter extraction with the given pattern.
    ///
    /// See [`Path`] for pattern syntax documentation.
    fn path(self, resource: &str) -> Path<Self>;
}

impl<E> PathExtractor for E
where
    E: Extractor,
{
    fn path(self, resource: &str) -> Path<Self> {
        Path {
            inner: self,
            resource: ResourceDef::from(resource),
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

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let mut path = actix_router::Path::new(subject.parts().uri.path());
        self.resource.capture_match_info(&mut path);
        let mut matched_parts = path
            .iter()
            .map(|(key, value)| KeyPart::new(key, Some(value)))
            .collect::<Vec<_>>();
        let mut parts = self.inner.get(subject).await;
        parts.append(&mut matched_parts);
        parts
    }
}
