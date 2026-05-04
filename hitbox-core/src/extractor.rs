//! Cache key extraction from requests.
//!
//! This module provides the [`Extractor`] trait for extracting data from
//! requests to build cache keys.
//!
//! ## Overview
//!
//! Extractors pull relevant data from requests (like HTTP method, path,
//! query parameters) and produce [`KeyParts`] that form the cache key.
//! Multiple extractors can be chained to build complex cache keys.
//!
//! ## Composability
//!
//! Extractors are designed to be composed. Protocol-specific crates like
//! `hitbox-http` provide extractors for common request components that
//! can be combined to create precise cache keys.
//!
//! ## Example
//!
//! ```ignore
//! use hitbox_core::{Extractor, KeyParts, KeyPart};
//!
//! #[derive(Debug)]
//! struct MethodExtractor;
//!
//! #[async_trait::async_trait]
//! impl Extractor for MethodExtractor {
//!     type Subject = HttpRequest;
//!
//!     async fn get(&self, request: Self::Subject) -> KeyParts<Self::Subject> {
//!         let mut parts = KeyParts::new(request);
//!         parts.push(KeyPart::new("method", Some(request.method().as_str())));
//!         parts
//!     }
//! }
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;

use crate::{KeyPart, KeyParts};

/// Trait for extracting cache key components from a subject.
///
/// Extractors are the mechanism for building cache keys from requests.
/// They are **protocol-agnostic** - the same trait works for HTTP requests,
/// gRPC messages, or any other protocol type.
///
/// # Type Parameters
///
/// The `Subject` associated type defines what this extractor processes.
/// Typically this is a request type from which cache key components
/// are extracted.
///
/// # Ownership
///
/// The `get` method takes ownership of the subject and returns it wrapped
/// in [`KeyParts`] along with the extracted key components. This allows
/// extractors to be chained without cloning.
///
/// # Blanket Implementations
///
/// This trait is implemented for:
/// - `&T` where `T: Extractor`
/// - `Box<T>` where `T: Extractor`
/// - `Arc<T>` where `T: Extractor`
#[async_trait]
pub trait Extractor {
    /// The type from which cache key components are extracted.
    type Subject;

    /// Extract cache key components from the subject.
    ///
    /// Returns a [`KeyParts`] containing the subject and accumulated key parts.
    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject>;
}

#[async_trait]
impl<T> Extractor for &T
where
    T: Extractor + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn get(&self, subject: T::Subject) -> KeyParts<T::Subject> {
        self.get(subject).await
    }
}

#[async_trait]
impl<T> Extractor for Box<T>
where
    T: Extractor + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn get(&self, subject: T::Subject) -> KeyParts<T::Subject> {
        self.as_ref().get(subject).await
    }
}

#[async_trait]
impl<T> Extractor for Arc<T>
where
    T: Extractor + Send + Sync + ?Sized,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn get(&self, subject: T::Subject) -> KeyParts<T::Subject> {
        self.as_ref().get(subject).await
    }
}

/// Extractor that emits a fixed list of [`KeyPart`]s regardless of subject.
///
/// Useful for embedding constant data in cache keys — environment markers,
/// service identifiers, deployment tags — and, when wrapped via
/// [`ExtractorExt::as_tag`](crate::tag::ExtractorExt::as_tag), for
/// constant-tag invalidation namespaces.
///
/// `Subject` is a phantom slot — the extractor never inspects the subject.
/// The `PhantomData<fn() -> S>` variance keeps `StaticExtractor<S>` `Send + Sync`
/// even when `S` isn't.
///
/// # Examples
///
/// As a key extractor:
/// ```ignore
/// use hitbox_core::{Extractor, KeyPart, StaticExtractor};
///
/// let ext = StaticExtractor::<()>::new(vec![
///     KeyPart::new("env", Some("prod")),
///     KeyPart::new("region", Some("eu-west-1")),
/// ]);
/// ```
///
/// As a tag extractor (via `as_tag`):
/// ```ignore
/// use hitbox_core::{KeyPart, StaticExtractor};
/// use hitbox_core::tag::ExtractorExt;
///
/// // Each KeyPart becomes a `"key=value"` CacheTag.
/// let tag_ext = StaticExtractor::<()>::new(vec![
///     KeyPart::new("user", Some("42")),
///     KeyPart::new("region", Some("eu")),
/// ]).as_tag();
/// ```
pub struct StaticExtractor<S> {
    parts: Vec<KeyPart>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> StaticExtractor<S> {
    /// Create a static extractor that emits `parts` for every subject.
    pub fn new(parts: Vec<KeyPart>) -> Self {
        Self {
            parts,
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for StaticExtractor<S> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<S> std::fmt::Debug for StaticExtractor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticExtractor")
            .field("parts", &self.parts)
            .finish()
    }
}

#[async_trait]
impl<S> Extractor for StaticExtractor<S>
where
    S: Send,
{
    type Subject = S;

    async fn get(&self, subject: S) -> KeyParts<S> {
        let mut key_parts = KeyParts::new(subject);
        for part in &self.parts {
            key_parts.push(part.clone());
        }
        key_parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag::ExtractorExt;

    #[tokio::test]
    async fn static_extractor_emits_configured_parts() {
        let ext = StaticExtractor::<u32>::new(vec![
            KeyPart::new("env", Some("prod")),
            KeyPart::new("region", Some("eu")),
        ]);

        let key_parts = ext.get(7).await;
        let (subject, key) = key_parts.into_cache_key();

        assert_eq!(subject, 7);
        let parts: Vec<_> = key.parts().collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].key(), "env");
        assert_eq!(parts[0].value(), Some("prod"));
        assert_eq!(parts[1].key(), "region");
        assert_eq!(parts[1].value(), Some("eu"));
    }

    #[tokio::test]
    async fn static_extractor_default_emits_no_parts() {
        let ext = StaticExtractor::<u32>::default();
        let key_parts = ext.get(0).await;
        let (_subject, key) = key_parts.into_cache_key();
        assert_eq!(key.parts().count(), 0);
    }

    #[tokio::test]
    async fn static_extractor_via_as_tag_yields_key_value_tags() {
        // KeyPart with Some value → CacheTag is "key=value".
        let tag_ext = StaticExtractor::<u32>::new(vec![
            KeyPart::new("user", Some("42")),
            KeyPart::new("region", Some("eu")),
        ])
        .as_tag();

        use crate::tag::TagExtractor;
        let (subject, tags) = tag_ext.extract_tags(123).await;
        assert_eq!(subject, 123);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str(), "user=42");
        assert_eq!(tags[1].as_str(), "region=eu");
    }
}
