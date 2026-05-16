//! Cache tag types for group invalidation.
//!
//! This module provides types for tagging cached entries and extracting tags:
//!
//! - [`CacheTag`] - A single cache tag for group invalidation
//! - [`CacheTags`] - Request and response tags stored with a cache entry
//! - [`TagExtractor`] - Trait for deriving tags from requests or responses
//! - [`TagAdapter`] - Adapter to reuse any [`Extractor`] as a [`TagExtractor`]
//!
//! ## Tag-Based Invalidation
//!
//! Tags allow invalidating groups of cache entries without knowing their exact keys.
//! For example, tagging all entries related to a user allows invalidating them all
//! when the user's data changes:
//!
//! ```
//! use hitbox_core::tag::CacheTag;
//!
//! let tag = CacheTag::new("user:42");
//! assert_eq!(tag.as_str(), "user:42");
//! ```
//!
//! ## Tag Extraction
//!
//! Tags can be derived from requests (known before cache read, enabling parallel
//! prefetch) or from responses (stored in the cache entry for post-read checks).
//!
//! Existing [`Extractor`] implementations can be reused as tag extractors via
//! [`TagAdapter`], which converts [`KeyPart`]s into [`CacheTag`]s.
//!
//! [`Extractor`]: crate::Extractor
//! [`KeyPart`]: crate::KeyPart

use std::sync::Arc;

use async_trait::async_trait;
use smol_str::SmolStr;

use crate::Extractor;
use crate::key::{CacheKey, KeyPart};

/// A cache tag for group invalidation.
///
/// Tags are lightweight string identifiers associated with cache entries.
/// Invalidating a tag marks all entries with that tag as stale.
///
/// Uses [`SmolStr`] internally for small string optimization — tags up to
/// 23 bytes are stored inline without heap allocation.
///
/// # Example
///
/// ```
/// use hitbox_core::tag::CacheTag;
///
/// let tag = CacheTag::new("user:42");
/// assert_eq!(tag.as_str(), "user:42");
///
/// // Tags can be compared
/// assert_eq!(CacheTag::new("user:42"), CacheTag::new("user:42"));
/// assert_ne!(CacheTag::new("user:42"), CacheTag::new("user:43"));
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CacheTag(SmolStr);

impl CacheTag {
    /// Creates a new cache tag.
    pub fn new(tag: impl Into<SmolStr>) -> Self {
        CacheTag(tag.into())
    }

    /// Returns the tag as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Converts this tag into a [`CacheKey`] for storing invalidation timestamps.
    ///
    /// Uses the given prefix as the key namespace. The tag value is stored
    /// as a key part.
    /// Converts this tag into a [`CacheKey`](crate::CacheKey) for storing
    /// invalidation timestamps.
    pub fn to_cache_key(&self, prefix: &str) -> CacheKey {
        CacheKey::new(prefix, 0, vec![KeyPart::new("tag", Some(self.as_str()))])
    }
}

impl std::fmt::Display for CacheTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&crate::KeyPart> for CacheTag {
    /// Converts a [`KeyPart`](crate::KeyPart) into a [`CacheTag`].
    ///
    /// Format: `"key=value"` or `"key"` (if value is `None`).
    fn from(part: &crate::KeyPart) -> Self {
        match part.value() {
            Some(value) => CacheTag::new(format!("{}={}", part.key(), value)),
            None => CacheTag::new(part.key()),
        }
    }
}

/// Request and response tags stored with a cache entry.
///
/// Each side is `Option<Vec<CacheTag>>` so we can distinguish:
/// - `None`: tag extractor was not configured for this side
/// - `Some(vec![])`: extractor ran, produced no tags
/// - `Some(vec![...])`: extractor produced tags
///
/// This matters for forensics / drift detection — empty vs unconfigured are
/// different states. For invalidation correctness, both encodings produce
/// the same behavior.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CacheTags {
    /// Tags derived from the request at write time.
    pub request: Option<Vec<CacheTag>>,
    /// Tags derived from the response at write time.
    pub response: Option<Vec<CacheTag>>,
}

impl CacheTags {
    /// Creates empty tags (both sides `None`).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates `CacheTags` with both request and response tag lists.
    pub fn new(request: Vec<CacheTag>, response: Vec<CacheTag>) -> Self {
        Self {
            request: Some(request),
            response: Some(response),
        }
    }

    /// Creates `CacheTags` with only request tags configured.
    pub fn request_only(request: Vec<CacheTag>) -> Self {
        Self {
            request: Some(request),
            response: None,
        }
    }

    /// Creates `CacheTags` with only response tags configured.
    pub fn response_only(response: Vec<CacheTag>) -> Self {
        Self {
            request: None,
            response: Some(response),
        }
    }

    /// Returns true if neither side has any tag info (both `None` or empty).
    pub fn is_empty(&self) -> bool {
        self.request.as_ref().is_none_or(Vec::is_empty)
            && self.response.as_ref().is_none_or(Vec::is_empty)
    }
}

/// Trait for extracting cache tags from a subject.
///
/// Mirrors the [`Extractor`] trait: async, takes ownership of the subject,
/// and returns it alongside the extracted tags. This allows tag extractors
/// to consume request/response bodies if needed (e.g., JQ body extraction).
///
/// [`Extractor`]: crate::Extractor
///
/// # Example
///
/// ```ignore
/// use hitbox_core::tag::{CacheTag, TagExtractor};
///
/// struct UserTagExtractor;
///
/// #[async_trait::async_trait]
/// impl TagExtractor for UserTagExtractor {
///     type Subject = u64;
///
///     async fn extract_tags(&self, user_id: u64) -> (u64, Vec<CacheTag>) {
///         (user_id, vec![CacheTag::new(format!("user:{user_id}"))])
///     }
/// }
/// ```
#[async_trait]
pub trait TagExtractor {
    /// The type from which tags are extracted.
    type Subject;

    /// Extract cache tags from the subject.
    ///
    /// Takes ownership of the subject and returns it alongside the tags,
    /// mirroring the [`Extractor`](crate::Extractor) ownership pattern.
    async fn extract_tags(&self, subject: Self::Subject) -> (Self::Subject, Vec<CacheTag>);
}

#[async_trait]
impl<T> TagExtractor for &T
where
    T: TagExtractor + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn extract_tags(&self, subject: T::Subject) -> (T::Subject, Vec<CacheTag>) {
        (**self).extract_tags(subject).await
    }
}

#[async_trait]
impl<T> TagExtractor for Box<T>
where
    T: TagExtractor + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn extract_tags(&self, subject: T::Subject) -> (T::Subject, Vec<CacheTag>) {
        self.as_ref().extract_tags(subject).await
    }
}

#[async_trait]
impl<T> TagExtractor for Arc<T>
where
    T: TagExtractor + Send + Sync + ?Sized,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn extract_tags(&self, subject: T::Subject) -> (T::Subject, Vec<CacheTag>) {
        self.as_ref().extract_tags(subject).await
    }
}

/// Adapter that wraps an [`Extractor`] to use it as a [`TagExtractor`].
///
/// Converts each [`KeyPart`](crate::KeyPart) produced by the extractor into
/// a [`CacheTag`]. This allows reusing existing extractor implementations
/// (e.g., HTTP method, path extractors) for tag-based invalidation.
///
/// # Example
///
/// ```ignore
/// use hitbox_core::tag::TagAdapter;
/// use hitbox_http::extractors::MethodExtractor;
///
/// // Reuse the HTTP method extractor as a tag extractor
/// let tag_extractor = TagAdapter::new(MethodExtractor::new(MethodConfig::new()));
/// ```
pub struct TagAdapter<E>(E);

impl<E> TagAdapter<E> {
    /// Wraps an extractor as a tag extractor.
    pub fn new(extractor: E) -> Self {
        TagAdapter(extractor)
    }
}

#[async_trait]
impl<E> TagExtractor for TagAdapter<E>
where
    E: Extractor + Send + Sync,
    E::Subject: Send,
{
    type Subject = E::Subject;

    async fn extract_tags(&self, subject: E::Subject) -> (E::Subject, Vec<CacheTag>) {
        let key_parts = self.0.get(subject).await;
        let (subject, cache_key) = key_parts.into_cache_key();
        let tags = cache_key.parts().map(CacheTag::from).collect();
        (subject, tags)
    }
}

/// Extension trait for converting any [`Extractor`] into a [`TagExtractor`].
///
/// This is automatically implemented for all [`Extractor`] types.
///
/// # Example
///
/// ```ignore
/// use hitbox_core::tag::ExtractorExt;
///
/// let tag_extractor = request::extractor::<Body>().path().as_tag();
/// ```
///
/// [`Extractor`]: crate::Extractor
pub trait ExtractorExt: Extractor + Sized {
    /// Wraps this extractor as a [`TagExtractor`].
    ///
    /// Each [`KeyPart`](crate::KeyPart) produced by the extractor is converted
    /// into a [`CacheTag`].
    fn as_tag(self) -> TagAdapter<Self> {
        TagAdapter::new(self)
    }
}

impl<T: Extractor> ExtractorExt for T {}

/// Tag extractor that runs a sequence of inner extractors and concatenates
/// their tags.
///
/// Useful for composing multiple [`TagExtractor`]s — for example a static
/// list of literal tags plus a [`TagAdapter`] over a request `Path`/`Header`
/// extractor — into a single tag extractor that the FSM can consume.
///
/// Subjects are threaded through each inner extractor in order: each one
/// receives the subject value, may produce tags, and returns the subject
/// for the next extractor in the chain.
pub struct ChainTagExtractor<S> {
    inner: Vec<Box<dyn TagExtractor<Subject = S> + Send + Sync>>,
}

impl<S> ChainTagExtractor<S> {
    /// Create a chain from an explicit vector of boxed inner tag extractors.
    pub fn new(inner: Vec<Box<dyn TagExtractor<Subject = S> + Send + Sync>>) -> Self {
        Self { inner }
    }
}

impl<S> std::fmt::Debug for ChainTagExtractor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainTagExtractor")
            .field("count", &self.inner.len())
            .finish()
    }
}

#[async_trait]
impl<S> TagExtractor for ChainTagExtractor<S>
where
    S: Send,
{
    type Subject = S;

    async fn extract_tags(&self, mut subject: S) -> (S, Vec<CacheTag>) {
        let mut tags = Vec::new();
        for ext in &self.inner {
            let (returned_subject, mut new_tags) = ext.extract_tags(subject).await;
            subject = returned_subject;
            tags.append(&mut new_tags);
        }
        (subject, tags)
    }
}

/// A no-op tag extractor that produces no tags.
///
/// Used as the default when no tag extraction is configured.
///
/// Uses `PhantomData<fn() -> S>` so that `NeutralTagExtractor<S>` is always
/// `Send + Sync`, even when `S` is not. The phantom variance is "produces S"
/// rather than "contains S" — appropriate for an extractor that never holds one.
pub struct NeutralTagExtractor<S>(std::marker::PhantomData<fn() -> S>);

impl<S> Default for NeutralTagExtractor<S> {
    fn default() -> Self {
        NeutralTagExtractor(std::marker::PhantomData)
    }
}

#[async_trait]
impl<S> TagExtractor for NeutralTagExtractor<S>
where
    S: Send,
{
    type Subject = S;

    async fn extract_tags(&self, subject: S) -> (S, Vec<CacheTag>) {
        (subject, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tag extractor that emits a fixed list of tags. Test-only — production
    /// code should use [`StaticExtractor`](crate::StaticExtractor) plus
    /// [`ExtractorExt::as_tag`].
    struct FixedTags(Vec<CacheTag>);

    #[async_trait]
    impl TagExtractor for FixedTags {
        type Subject = u32;

        async fn extract_tags(&self, subject: u32) -> (u32, Vec<CacheTag>) {
            (subject, self.0.clone())
        }
    }

    #[tokio::test]
    async fn chain_concatenates_tags_in_order() {
        let chain = ChainTagExtractor::<u32>::new(vec![
            Box::new(FixedTags(vec![CacheTag::new("a"), CacheTag::new("b")])),
            Box::new(FixedTags(vec![CacheTag::new("c")])),
        ]);

        let (subject, tags) = chain.extract_tags(7).await;
        assert_eq!(subject, 7);
        let strs: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
        assert_eq!(strs, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn empty_chain_produces_no_tags() {
        let chain: ChainTagExtractor<u32> = ChainTagExtractor::new(Vec::new());
        let (_subject, tags) = chain.extract_tags(0).await;
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn chain_threads_subject_through_each_link() {
        // Even a no-tag link must still pass the subject onward unchanged.
        let chain = ChainTagExtractor::<u32>::new(vec![
            Box::new(NeutralTagExtractor::<u32>::default()),
            Box::new(FixedTags(vec![CacheTag::new("only-tag")])),
        ]);

        let (subject, tags) = chain.extract_tags(99).await;
        assert_eq!(subject, 99);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), "only-tag");
    }
}
