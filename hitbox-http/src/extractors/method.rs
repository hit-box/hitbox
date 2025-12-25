use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use super::NeutralExtractor;
use crate::CacheableHttpRequest;

/// Extracts the HTTP method as a cache key part.
///
/// Adds a key part with name `"method"` and the method as value (e.g., `"GET"`, `"POST"`).
/// Use this as the starting point for extractor chains.
///
/// # Type Parameters
///
/// * `E` - The inner extractor to chain with. Use [`Method::new`] to start
///   a new extractor chain (uses [`NeutralExtractor`] internally), or use the
///   [`MethodExtractor`] extension trait to chain onto an existing extractor.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{Method, path::PathExtractor, query::QueryExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Path, query::Query};
/// let extractor = Method::new()
///     .path("/users/{user_id}")
///     .query("page".to_string());
/// # let _: &Query<Path<Method<NeutralExtractor<Empty<Bytes>>>>> = &extractor;
/// ```
///
/// # Key Parts Generated
///
/// Generates a single key part: `method={METHOD}` where `{METHOD}` is the
/// uppercase HTTP method name (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, etc.).
#[derive(Debug)]
pub struct Method<E> {
    inner: E,
}

impl<S> Method<NeutralExtractor<S>> {
    /// Creates a method extractor as the starting point for cache key generation.
    ///
    /// Adds a key part with name `"method"` and the HTTP method as value
    /// (e.g., `"GET"`, `"POST"`). Chain additional extractors to build
    /// a complete cache key.
    ///
    /// Chain onto existing extractors using [`MethodExtractor::method`] instead
    /// if you already have an extractor chain.
    pub fn new() -> Self {
        Self {
            inner: NeutralExtractor::new(),
        }
    }
}

impl<S> Default for Method<NeutralExtractor<S>> {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for adding method extraction to an extractor chain.
///
/// # For Callers
///
/// Chain this after [`Method::new()`] or any other extractor to add the HTTP
/// method to your cache key. The method is added as a key part with name
/// `"method"` and value like `"GET"` or `"POST"`.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Extractor`]
/// types. You don't need to implement it manually.
pub trait MethodExtractor: Sized {
    /// Adds HTTP method extraction to the chain.
    fn method(self) -> Method<Self>;
}

impl<E> MethodExtractor for E
where
    E: Extractor,
{
    fn method(self) -> Method<Self> {
        Method { inner: self }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Method<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let method = subject.parts().method.to_string();
        let mut parts = self.inner.get(subject).await;
        parts.push(KeyPart::new("method", Some(method)));
        parts
    }
}
