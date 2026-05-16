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
/// lifetime.
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

    /// Records current cache capacity metrics.
    ///
    /// Updates the `hitbox_moka_entries` and `hitbox_moka_size_bytes` gauges
    /// with the current cache state. The backend's label is used as the
    /// `backend` metric label.
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
        crate::metrics::record_capacity(
            self.label.as_str(),
            self.entry_count(),
            self.weighted_size(),
        );
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
        self.record_metrics();
        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let value = self.cache.remove(key).await;
        self.record_metrics();
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
