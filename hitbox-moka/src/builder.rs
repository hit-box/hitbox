//! Builder for configuring [`MokaBackend`].

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use moka::Expiry;
use moka::future::{Cache, CacheBuilder};
use moka::policy::EvictionPolicy;

use crate::backend::MokaBackend;
use hitbox::{BackendLabel, CacheKey, CacheValue, Raw};
use hitbox_core::CacheTag;
use hitbox_backend::format::{Format, JsonFormat};
use hitbox_backend::{CacheKeyFormat, Compressor, PassthroughCompressor};

/// Custom expiration policy that calculates TTL from [`CacheValue::expire`] timestamps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Expiration;

impl Expiry<CacheKey, CacheValue<Raw>> for Expiration {
    fn expire_after_create(
        &self,
        _key: &CacheKey,
        value: &CacheValue<Raw>,
        _created_at: Instant,
    ) -> Option<Duration> {
        Self::calculate_ttl(value)
    }

    fn expire_after_update(
        &self,
        _key: &CacheKey,
        value: &CacheValue<Raw>,
        _updated_at: Instant,
        _duration_until_expiry: Option<Duration>,
    ) -> Option<Duration> {
        // IMPORTANT: Always use the NEW value's expiration time.
        //
        // Moka's default `expire_after_update` returns `duration_until_expiry`,
        // which preserves the OLD expiration time. This causes premature expiration
        // when updating a cache entry with a new (longer) TTL.
        Self::calculate_ttl(value)
    }
}

impl Expiration {
    fn calculate_ttl(value: &CacheValue<Raw>) -> Option<Duration> {
        value.expire().map(|expiration| {
            let delta = expiration - Utc::now();
            let millis = delta.num_milliseconds();
            if millis <= 0 {
                Duration::ZERO
            } else {
                Duration::from_millis(millis as u64)
            }
        })
    }
}

/// Builds the dedicated, unbounded tag invalidation store.
///
/// Tag records (`CacheTag` → `DateTime<Utc>`) are kept apart from the data
/// cache so data write pressure can never evict an invalidation record.
/// No `max_capacity` is set, so the store never evicts; there is also no
/// expiry — records persist for the process lifetime.
fn build_tag_cache() -> Cache<CacheTag, DateTime<Utc>> {
    Cache::builder().build()
}

/// Marker type: capacity has not been configured yet.
///
/// This is the initial state of a [`MokaBackendBuilder`]. You must call either
/// [`max_entries()`](MokaBackendBuilder::max_entries) or
/// [`max_bytes()`](MokaBackendBuilder::max_bytes) before calling `build()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoCapacity;

/// Marker type: entry-count capacity has been configured.
///
/// The cache will hold at most `n` entries, evicting least recently used
/// entries when capacity is exceeded.
#[derive(Debug, Clone, Copy)]
pub struct EntryCapacity(pub(crate) u64);

/// Marker type: byte-based capacity has been configured.
///
/// The cache will use at most `n` bytes (approximate), evicting least recently
/// used entries when the memory budget is exceeded.
#[derive(Debug, Clone, Copy)]
pub struct ByteCapacity(pub(crate) u64);

/// Builder for creating and configuring a [`MokaBackend`].
///
/// Use [`MokaBackend::builder`] to create a new builder instance.
///
/// # Capacity Configuration (Required)
///
/// You must configure capacity using exactly one of:
/// - [`max_entries(n)`](Self::max_entries) - limit by entry count
/// - [`max_bytes(n)`](Self::max_bytes) - limit by approximate memory usage
///
/// These methods use the typestate pattern to enforce compile-time guarantees:
/// - `build()` is only available after setting capacity
/// - You cannot set both entry and byte limits
///
/// # Examples
///
/// Entry-based capacity:
///
/// ```
/// use hitbox_moka::MokaBackend;
///
/// let backend = MokaBackend::builder()
///     .max_entries(10_000)
///     .build();
/// ```
///
/// Byte-based capacity (100 MB):
///
/// ```
/// use hitbox_moka::MokaBackend;
///
/// let backend = MokaBackend::builder()
///     .max_bytes(100 * 1024 * 1024)
///     .build();
/// ```
///
/// With custom configuration:
///
/// ```ignore
/// use hitbox_moka::MokaBackend;
/// use hitbox_backend::format::BincodeFormat;
/// use hitbox_backend::{CacheKeyFormat, GzipCompressor};
///
/// let backend = MokaBackend::builder()
///     .label("sessions")
///     .max_bytes(50_000_000)
///     .key_format(CacheKeyFormat::UrlEncoded)
///     .value_format(BincodeFormat)
///     .compressor(GzipCompressor::default())
///     .build();
/// ```
///
/// **Note:** `GzipCompressor` and `ZstdCompressor` require enabling the `gzip`
/// or `zstd` feature on `hitbox-backend`.
pub struct MokaBackendBuilder<Cap, S = JsonFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    capacity: Cap,
    key_format: CacheKeyFormat,
    serializer: S,
    compressor: C,
    label: BackendLabel,
    eviction_policy: Option<EvictionPolicy>,
}

impl MokaBackendBuilder<NoCapacity, JsonFormat, PassthroughCompressor> {
    /// Creates a new builder with no capacity configured.
    ///
    /// You must call [`max_entries()`](Self::max_entries) or
    /// [`max_bytes()`](Self::max_bytes) before calling `build()`.
    pub fn new() -> Self {
        Self {
            capacity: NoCapacity,
            key_format: CacheKeyFormat::Bitcode,
            serializer: JsonFormat,
            compressor: PassthroughCompressor,
            label: BackendLabel::new_static("moka"),
            eviction_policy: None,
        }
    }
}

impl Default for MokaBackendBuilder<NoCapacity, JsonFormat, PassthroughCompressor> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, C> MokaBackendBuilder<NoCapacity, S, C>
where
    S: Format,
    C: Compressor,
{
    /// Sets the maximum number of entries the cache can hold.
    ///
    /// When the cache exceeds this capacity, least recently used entries are
    /// evicted.
    ///
    /// # Example
    ///
    /// ```
    /// use hitbox_moka::MokaBackend;
    ///
    /// let backend = MokaBackend::builder()
    ///     .max_entries(10_000)
    ///     .build();
    /// ```
    pub fn max_entries(self, capacity: u64) -> MokaBackendBuilder<EntryCapacity, S, C> {
        MokaBackendBuilder {
            capacity: EntryCapacity(capacity),
            key_format: self.key_format,
            serializer: self.serializer,
            compressor: self.compressor,
            label: self.label,
            eviction_policy: self.eviction_policy,
        }
    }

    /// Sets the maximum memory budget in bytes.
    ///
    /// The cache will use approximately this many bytes for stored values,
    /// evicting least recently used entries when the budget is exceeded.
    ///
    /// The byte count includes:
    /// - Serialized value data
    /// - Fixed overhead estimate for keys and metadata (~112 bytes per entry)
    ///
    /// # Example
    ///
    /// ```
    /// use hitbox_moka::MokaBackend;
    ///
    /// // 100 MB cache
    /// let backend = MokaBackend::builder()
    ///     .max_bytes(100 * 1024 * 1024)
    ///     .build();
    /// ```
    pub fn max_bytes(self, bytes: u64) -> MokaBackendBuilder<ByteCapacity, S, C> {
        MokaBackendBuilder {
            capacity: ByteCapacity(bytes),
            key_format: self.key_format,
            serializer: self.serializer,
            compressor: self.compressor,
            label: self.label,
            eviction_policy: self.eviction_policy,
        }
    }
}

impl<Cap, S, C> MokaBackendBuilder<Cap, S, C>
where
    S: Format,
    C: Compressor,
{
    /// Sets a custom label for this backend.
    ///
    /// The label identifies this backend in multi-tier cache compositions and
    /// appears in metrics and debug output.
    ///
    /// # Default
    ///
    /// `"moka"`
    pub fn label(mut self, label: impl Into<BackendLabel>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the cache key serialization format.
    ///
    /// The key format determines how [`CacheKey`] values are serialized for
    /// storage. This affects key size and debuggability.
    ///
    /// # Default
    ///
    /// [`CacheKeyFormat::Bitcode`]
    ///
    /// # Options
    ///
    /// | Format | Size | Human-readable |
    /// |--------|------|----------------|
    /// | [`Bitcode`](CacheKeyFormat::Bitcode) | Compact | No |
    /// | [`UrlEncoded`](CacheKeyFormat::UrlEncoded) | Larger | Yes |
    ///
    /// [`CacheKey`]: hitbox_core::CacheKey
    pub fn key_format(mut self, format: CacheKeyFormat) -> Self {
        self.key_format = format;
        self
    }

    /// Sets the eviction policy for the cache.
    ///
    /// The eviction policy determines how entries are selected for removal when
    /// the cache reaches capacity.
    ///
    /// # Default
    ///
    /// - **Entry-based capacity** ([`max_entries`]): [`EvictionPolicy::tiny_lfu()`] -
    ///   combines LRU eviction with LFU admission for optimal hit rates
    /// - **Byte-based capacity** ([`max_bytes`]): [`EvictionPolicy::lru()`] -
    ///   pure LRU for predictable eviction behavior with weighted entries
    ///
    /// # Options
    ///
    /// | Policy | Description | Best for |
    /// |--------|-------------|----------|
    /// | [`tiny_lfu()`](EvictionPolicy::tiny_lfu) | LRU eviction + LFU admission | General caching, web workloads |
    /// | [`lru()`](EvictionPolicy::lru) | Pure least-recently-used | Recency-biased, streaming data |
    ///
    /// # Example
    ///
    /// ```
    /// use hitbox_moka::{MokaBackend, EvictionPolicy};
    ///
    /// // Use TinyLFU with byte-based capacity (overriding default LRU)
    /// let backend = MokaBackend::builder()
    ///     .max_bytes(100 * 1024 * 1024)
    ///     .eviction_policy(EvictionPolicy::tiny_lfu())
    ///     .build();
    /// ```
    ///
    /// [`max_entries`]: Self::max_entries
    /// [`max_bytes`]: Self::max_bytes
    pub fn eviction_policy(mut self, policy: EvictionPolicy) -> Self {
        self.eviction_policy = Some(policy);
        self
    }

    /// Sets the cache value serialization format.
    ///
    /// The value format determines how cached data is serialized before storage.
    ///
    /// # Default
    ///
    /// [`JsonFormat`]
    ///
    /// # Options
    ///
    /// | Format | Speed | Size | Human-readable |
    /// |--------|-------|------|----------------|
    /// | [`JsonFormat`] | Slow | Large | Yes |
    /// | [`BincodeFormat`](hitbox_backend::format::BincodeFormat) | Fast | Compact | No |
    /// | [`RonFormat`](hitbox_backend::format::RonFormat) | Medium | Medium | Yes |
    pub fn value_format<NewS>(self, serializer: NewS) -> MokaBackendBuilder<Cap, NewS, C>
    where
        NewS: Format,
    {
        MokaBackendBuilder {
            capacity: self.capacity,
            key_format: self.key_format,
            serializer,
            compressor: self.compressor,
            label: self.label,
            eviction_policy: self.eviction_policy,
        }
    }

    /// Sets the compression strategy for cache values.
    ///
    /// Compression reduces memory usage at the cost of CPU time. For in-memory
    /// caches like Moka, compression is typically **not recommended** since
    /// memory access is fast and compression adds latency.
    ///
    /// # Default
    ///
    /// [`PassthroughCompressor`] (no compression)
    ///
    /// # Options
    ///
    /// | Compressor | Ratio | Speed | Feature flag |
    /// |------------|-------|-------|--------------|
    /// | [`PassthroughCompressor`] | None | Fastest | — |
    /// | [`GzipCompressor`] | Good | Medium | `gzip` |
    /// | [`ZstdCompressor`] | Best | Fast | `zstd` |
    ///
    /// # When to Use Compression
    ///
    /// - Large cached values (>10KB)
    /// - Memory-constrained environments
    /// - When composing with network backends (compression done once, reused)
    ///
    /// [`PassthroughCompressor`]: hitbox_backend::PassthroughCompressor
    /// [`GzipCompressor`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.GzipCompressor.html
    /// [`ZstdCompressor`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.ZstdCompressor.html
    pub fn compressor<NewC>(self, compressor: NewC) -> MokaBackendBuilder<Cap, S, NewC>
    where
        NewC: Compressor,
    {
        MokaBackendBuilder {
            capacity: self.capacity,
            key_format: self.key_format,
            serializer: self.serializer,
            compressor,
            label: self.label,
            eviction_policy: self.eviction_policy,
        }
    }
}

impl<S, C> MokaBackendBuilder<EntryCapacity, S, C>
where
    S: Format,
    C: Compressor,
{
    /// Builds the [`MokaBackend`] with entry-count based capacity.
    ///
    /// Consumes the builder and returns a fully configured backend ready for use.
    pub fn build(self) -> MokaBackend<S, C> {
        let policy = self
            .eviction_policy
            .unwrap_or_else(EvictionPolicy::tiny_lfu);
        let cache: Cache<CacheKey, CacheValue<Raw>> = CacheBuilder::new(self.capacity.0)
            .eviction_policy(policy)
            .expire_after(Expiration)
            .build();

        MokaBackend {
            cache,
            tags: build_tag_cache(),
            key_format: self.key_format,
            serializer: self.serializer,
            compressor: self.compressor,
            label: self.label,
        }
    }
}

impl<S, C> MokaBackendBuilder<ByteCapacity, S, C>
where
    S: Format + 'static,
    C: Compressor + 'static,
{
    /// Builds the [`MokaBackend`] with byte-based capacity.
    ///
    /// Consumes the builder and returns a fully configured backend ready for use.
    /// The cache will use a weigher function to track approximate memory usage.
    ///
    /// Note: Default eviction policy is LRU (not TinyLFU) to ensure predictable
    /// eviction behavior with weighted capacity. TinyLFU's admission policy can
    /// reject new entries even when eviction could make room. Override with
    /// [`eviction_policy()`](MokaBackendBuilder::eviction_policy) if needed.
    pub fn build(self) -> MokaBackend<S, C> {
        let policy = self.eviction_policy.unwrap_or_else(EvictionPolicy::lru);
        let cache: Cache<CacheKey, CacheValue<Raw>> = CacheBuilder::new(self.capacity.0)
            .weigher(Self::byte_weigher)
            .eviction_policy(policy)
            .expire_after(Expiration)
            .build();

        MokaBackend {
            cache,
            tags: build_tag_cache(),
            key_format: self.key_format,
            serializer: self.serializer,
            compressor: self.compressor,
            label: self.label,
        }
    }

    /// Weigher function that calculates the approximate byte cost of a cache entry.
    ///
    /// Uses the precalculated `memory_size()` methods on `CacheKey` and `CacheValue`
    /// which account for struct overhead and variable-length content.
    fn byte_weigher(key: &CacheKey, value: &CacheValue<Raw>) -> u32 {
        (key.memory_size() + value.memory_size()).min(u32::MAX as usize) as u32
    }
}
