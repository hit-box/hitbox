//! Moka backend implementation.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::future::join_all;
use hitbox::{BackendLabel, CacheKey, CacheValue, Raw};
use hitbox_backend::Backend;
use hitbox_backend::format::{Format, JsonFormat};
use hitbox_backend::{
    BackendResult, CacheKeyFormat, Compressor, DeleteStatus, PassthroughCompressor,
};
use hitbox_core::CacheTag;
use moka::future::Cache;

use crate::metrics::{record_capacity, record_tag_records};

/// In-memory cache backend powered by Moka.
///
/// `MokaBackend` provides a high-performance, concurrent in-memory cache with
/// automatic entry expiration. It uses Moka's async cache internally, which
/// offers lock-free reads and fine-grained locking for writes.
///
/// # Type Parameters
///
/// * `S` - Serialization format for cache values. Implements [`Format`].
///   Default: [`JsonFormat`].
/// * `C` - Compression strategy for cache values. Implements [`Compressor`].
///   Default: [`PassthroughCompressor`] (no compression).
///
/// # Examples
///
/// Basic usage with defaults:
///
/// ```
/// use hitbox_moka::MokaBackend;
///
/// let backend = MokaBackend::builder().max_entries(10_000).build();
/// ```
///
/// With custom serialization format:
///
/// ```
/// use hitbox_moka::MokaBackend;
/// use hitbox_backend::format::BincodeFormat;
///
/// let backend = MokaBackend::builder()
///     .max_entries(10_000)
///     .value_format(BincodeFormat)
///     .build();
/// ```
///
/// # Performance
///
/// - **Read operations**: Lock-free, O(1) average
/// - **Write operations**: Fine-grained locking, O(1) average
/// - **Memory**: Bounded by `max_capacity` entries
///
/// # Tag Invalidation Storage
///
/// Tag invalidation timestamps are kept in a **separate, unbounded** Moka
/// cache rather than sharing the data cache's eviction budget. This
/// matters for correctness: if a tag's invalidation record were evicted
/// under data write pressure, a subsequent read would see "no record" and
/// incorrectly treat the entry as never invalidated, serving stale data.
/// The dedicated tag store has no size cap and no expiry — records are
/// tiny (`CacheTag` → `DateTime<Utc>`) and persist for the process
/// lifetime. Its size is observable via [`tag_record_count()`] and, with the
/// `metrics` feature, the `hitbox_moka_tag_records` gauge.
///
/// # Caveats
///
/// - Data is **not persisted** — cache is lost on process restart
/// - Data is **not shared** across processes — use Redis for distributed caching
/// - Expiration is **best-effort** — expired entries may briefly remain readable
///   until Moka's background eviction runs
///
/// [`Format`]: hitbox_backend::format::Format
/// [`JsonFormat`]: hitbox_backend::format::JsonFormat
/// [`Compressor`]: hitbox_backend::Compressor
/// [`PassthroughCompressor`]: hitbox_backend::PassthroughCompressor
/// [`tag_record_count()`]: MokaBackend::tag_record_count
#[derive(Clone)]
pub struct MokaBackend<S = JsonFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    pub(crate) cache: Cache<CacheKey, CacheValue<Raw>>,
    /// Dedicated, unbounded store for tag invalidation timestamps.
    ///
    /// Kept separate from `cache` so data write pressure can never evict a
    /// tag invalidation record (which would silently break invalidation —
    /// see the type-level "Tag Invalidation Storage" docs).
    pub(crate) tags: Cache<CacheTag, DateTime<Utc>>,
    pub(crate) key_format: CacheKeyFormat,
    pub(crate) serializer: S,
    pub(crate) compressor: C,
    pub(crate) label: BackendLabel,
}

impl<S, C> MokaBackend<S, C>
where
    S: Format,
    C: Compressor,
{
    /// Returns a reference to the underlying Moka cache.
    ///
    /// This provides direct access to Moka-specific features like
    /// [`run_pending_tasks()`](Cache::run_pending_tasks) for synchronizing
    /// eviction in tests.
    pub fn cache(&self) -> &Cache<CacheKey, CacheValue<Raw>> {
        &self.cache
    }

    /// Returns the approximate number of entries in this cache.
    ///
    /// The value is approximate because concurrent operations may change
    /// the count between when it's calculated and when it's returned.
    /// Call [`run_pending_tasks()`](Cache::run_pending_tasks) first for
    /// more accurate results.
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Returns the approximate weighted size of this cache in bytes.
    ///
    /// This is only meaningful when the cache was created with [`max_bytes()`].
    /// For entry-count based caches, this returns the same as [`entry_count()`].
    ///
    /// The value is approximate because concurrent operations may change
    /// the size between when it's calculated and when it's returned.
    /// Call [`run_pending_tasks()`](Cache::run_pending_tasks) first for
    /// more accurate results.
    ///
    /// [`max_bytes()`]: crate::builder::MokaBackendBuilder::max_bytes
    /// [`entry_count()`]: Self::entry_count
    pub fn weighted_size(&self) -> u64 {
        self.cache.weighted_size()
    }

    /// Returns the approximate number of tag invalidation records.
    ///
    /// Tag records live in a dedicated, unbounded store (see the type-level
    /// "Tag Invalidation Storage" docs), so this grows with every distinct
    /// tag ever invalidated. Like [`entry_count()`], the value is approximate.
    ///
    /// [`entry_count()`]: Self::entry_count
    pub fn tag_record_count(&self) -> u64 {
        self.tags.entry_count()
    }

    /// Records current cache metrics.
    ///
    /// Updates the `hitbox_moka_entries`, `hitbox_moka_size_bytes` and
    /// `hitbox_moka_tag_records` gauges with the current cache state. The
    /// backend's label is used as the `backend` metric label.
    ///
    /// This method is a no-op when the `metrics` feature is disabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Call periodically or in a metrics endpoint handler
    /// backend.record_metrics();
    /// ```
    pub fn record_metrics(&self) {
        self.record_data_metrics();
        self.record_tag_metrics();
    }

    /// Updates the data-store gauges. Called on every data write and remove.
    fn record_data_metrics(&self) {
        record_capacity(
            self.label.as_str(),
            self.entry_count(),
            self.weighted_size(),
        );
    }

    /// Updates the tag-store gauge. Called on every tag invalidation, the
    /// store's only writer, so data writes don't pay for it.
    fn record_tag_metrics(&self) {
        record_tag_records(self.label.as_str(), self.tag_record_count());
    }
}

impl MokaBackend<JsonFormat, PassthroughCompressor> {
    /// Creates a new builder for `MokaBackend`.
    ///
    /// You must configure capacity using [`max_entries()`] or [`max_bytes()`] before
    /// calling `build()`.
    ///
    /// [`max_entries()`]: crate::builder::MokaBackendBuilder::max_entries
    /// [`max_bytes()`]: crate::builder::MokaBackendBuilder::max_bytes
    pub fn builder() -> crate::builder::MokaBackendBuilder<
        crate::builder::NoCapacity,
        JsonFormat,
        PassthroughCompressor,
    > {
        crate::builder::MokaBackendBuilder::new()
    }
}

#[async_trait]
impl<S, C> Backend for MokaBackend<S, C>
where
    S: Format + Send + Sync,
    C: Compressor + Send + Sync,
{
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        self.cache.get(key).await.map(Ok).transpose()
    }

    async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
        self.cache.insert(key.clone(), value).await;
        self.record_data_metrics();
        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let value = self.cache.remove(key).await;
        self.record_data_metrics();
        match value {
            Some(_) => Ok(DeleteStatus::Deleted(1)),
            None => Ok(DeleteStatus::Missing),
        }
    }

    fn label(&self) -> BackendLabel {
        self.label.clone()
    }

    fn value_format(&self) -> &dyn Format {
        &self.serializer
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &self.key_format
    }

    fn compressor(&self) -> &dyn Compressor {
        &self.compressor
    }

    /// Records a tag invalidation in the dedicated, unbounded tag store.
    ///
    /// Overrides the default `Backend` impl (which would route through
    /// `write` and share the data cache's eviction budget).
    async fn invalidate(&self, tag: &CacheTag) -> BackendResult<()> {
        self.tags.insert(tag.clone(), Utc::now()).await;
        self.record_tag_metrics();
        Ok(())
    }

    /// Looks up invalidation timestamps from the dedicated tag store.
    ///
    /// Lookups run concurrently. Tags with no record are absent from the
    /// returned map (never invalidated). Overrides the default `Backend`
    /// impl.
    async fn invalidated(
        &self,
        tags: &[CacheTag],
    ) -> BackendResult<HashMap<CacheTag, DateTime<Utc>>> {
        let lookups = tags
            .iter()
            .map(|tag| async move { self.tags.get(tag).await.map(|ts| (tag.clone(), ts)) });
        let result = join_all(lookups).await.into_iter().flatten().collect();
        Ok(result)
    }
}

// Explicit CacheBackend implementation using default trait methods
impl<S, C> hitbox_backend::CacheBackend for MokaBackend<S, C>
where
    S: Format + Send + Sync,
    C: Compressor + Send + Sync,
{
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use hitbox_core::KeyPart;

    use super::*;

    fn make_key(id: u32) -> CacheKey {
        CacheKey::new("test", 1, vec![KeyPart::new("id", Some(id.to_string()))])
    }

    #[tokio::test]
    async fn tag_record_count_tracks_distinct_invalidated_tags() {
        let backend = MokaBackend::builder().max_entries(100).build();

        // Data writes don't touch the tag store.
        for id in 0..10 {
            let value = CacheValue::new(Bytes::from_static(b"payload"), None, None);
            backend.write(&make_key(id), value).await.unwrap();
        }
        backend.tags.run_pending_tasks().await;
        assert_eq!(backend.tag_record_count(), 0);

        backend.invalidate(&CacheTag::new("user=42")).await.unwrap();
        backend
            .invalidate(&CacheTag::new("region=eu"))
            .await
            .unwrap();
        backend.tags.run_pending_tasks().await;
        assert_eq!(backend.tag_record_count(), 2);

        // Re-invalidating a tag refreshes its timestamp, not its record count.
        backend.invalidate(&CacheTag::new("user=42")).await.unwrap();
        backend.tags.run_pending_tasks().await;
        assert_eq!(backend.tag_record_count(), 2);
    }
}
