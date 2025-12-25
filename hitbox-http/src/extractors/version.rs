//! HTTP version extraction for cache keys.
//!
//! Provides [`Version`] extractor for including the HTTP protocol version
//! in cache keys.

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use super::NeutralExtractor;
use crate::CacheableHttpRequest;

/// Extracts the HTTP protocol version as a cache key part.
///
/// Generates a key part with name `"version"` and value like `"HTTP/1.1"` or `"HTTP/2"`.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{Method, version::VersionExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Version};
/// // Include version in cache key
/// let extractor = Method::new().version();
/// # let _: &Version<Method<NeutralExtractor<Empty<Bytes>>>> = &extractor;
/// ```
#[derive(Debug)]
pub struct Version<E> {
    inner: E,
}

impl<S> Version<NeutralExtractor<S>> {
    /// Creates a version extractor for cache key generation.
    ///
    /// Adds a key part with name `"version"` and the HTTP protocol version
    /// as value (e.g., `"HTTP/1.1"`, `"HTTP/2"`).
    ///
    /// Chain onto existing extractors using [`VersionExtractor::version`] instead
    /// if you already have an extractor chain.
    pub fn new() -> Self {
        Self {
            inner: NeutralExtractor::new(),
        }
    }
}

impl<S> Default for Version<NeutralExtractor<S>> {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for adding version extraction to an extractor chain.
///
/// # For Callers
///
/// Chain this to include the HTTP protocol version in your cache key.
/// The version is added as a key part with name `"version"` and value
/// like `"HTTP/1.1"` or `"HTTP/2"`.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Extractor`]
/// types. You don't need to implement it manually.
pub trait VersionExtractor: Sized {
    /// Adds HTTP version extraction to this extractor chain.
    fn version(self) -> Version<Self>;
}

impl<E> VersionExtractor for E
where
    E: Extractor,
{
    fn version(self) -> Version<Self> {
        Version { inner: self }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Version<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let version = format!("{:?}", subject.parts().version);
        let mut parts = self.inner.get(subject).await;
        parts.push(KeyPart::new("version", Some(version)));
        parts
    }
}
