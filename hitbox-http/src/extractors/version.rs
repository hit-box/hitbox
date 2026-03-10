//! HTTP version extraction for cache keys.
//!
//! Provides [`Version`] extractor for including the HTTP protocol version
//! in cache keys.

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use crate::CacheableHttpRequest;

/// Extracts the HTTP protocol version as a cache key part.
///
/// Generates a key part with name `"version"` and value like `"HTTP/1.1"` or `"HTTP/2"`.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, VersionConfig, VersionExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Method, Version};
/// // Include version in cache key
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .version(VersionConfig::new());
/// # let _: &Version<Method<NeutralExtractor<Empty<Bytes>>>> = &extractor;
/// ```
#[derive(Debug)]
pub struct Version<E> {
    inner: E,
}

/// Configuration for the version extractor.
///
/// This is a marker type with no configuration options — the HTTP version
/// is always extracted as-is.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, VersionConfig, VersionExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .version(VersionConfig::new());
/// ```
#[derive(Debug, Clone, Default)]
pub struct VersionConfig;

impl VersionConfig {
    /// Creates a new version extractor configuration.
    pub fn new() -> Self {
        VersionConfig
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
    fn version(self, config: VersionConfig) -> Version<Self>;
}

impl<E> VersionExtractor for E
where
    E: Extractor,
{
    fn version(self, _config: VersionConfig) -> Version<Self> {
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
    type Context = E::Context;

    async fn get(
        &self,
        subject: Self::Subject,
        ctx: &mut Self::Context,
    ) -> KeyParts<Self::Subject> {
        let version = format!("{:?}", subject.parts().version);
        let mut parts = self.inner.get(subject, ctx).await;
        parts.push(KeyPart::new("version", Some(version)));
        parts
    }
}
