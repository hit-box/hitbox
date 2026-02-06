use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use bincode::{
    config::standard as bincode_config,
    serde::{decode_from_slice, encode_to_vec},
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use feoxdb::{FeoxError, FeoxStore};
use hitbox_backend::format::{Format, JsonFormat};
use hitbox_backend::{
    Backend, BackendError, BackendResult, CacheKeyFormat, Compressor, DeleteStatus,
    PassthroughCompressor,
};
use hitbox_core::{BackendLabel, CacheKey, CacheValue, Raw};
use serde::{Deserialize, Serialize};

use crate::FeOxDbError;

#[derive(Serialize, Deserialize)]
struct SerializableCacheValue {
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
    stale: Option<DateTime<Utc>>,
    expire: Option<DateTime<Utc>>,
}

impl From<CacheValue<Raw>> for SerializableCacheValue {
    fn from(value: CacheValue<Raw>) -> Self {
        Self {
            data: value.data().to_vec(),
            stale: value.stale(),
            expire: value.expire(),
        }
    }
}

impl From<SerializableCacheValue> for CacheValue<Raw> {
    fn from(value: SerializableCacheValue) -> Self {
        CacheValue::new(Bytes::from(value.data), value.expire, value.stale)
    }
}

/// Disk-based cache backend using FeOxDB.
///
/// Use this when cache data must survive restarts or doesn't fit in memory.
/// For pure speed without persistence, prefer `MokaBackend`.
///
/// ```no_run
/// use hitbox_feoxdb::FeOxDbBackend;
///
/// // Persistent cache with defaults
/// let backend = FeOxDbBackend::builder()
///     .path("/var/cache/myapp")
///     .build()?;
///
/// // With resource limits
/// let backend = FeOxDbBackend::builder()
///     .path("/var/cache/myapp")
///     .max_file_size(10 * 1024 * 1024 * 1024)  // 10 GB
///     .max_memory(256 * 1024 * 1024)           // 256 MB
///     .build()?;
/// # Ok::<(), hitbox_feoxdb::FeOxDbError>(())
/// ```
///
/// Cloning is cheap — clones share the same underlying database.
#[derive(Clone)]
pub struct FeOxDbBackend<S = JsonFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    store: Arc<FeoxStore>,
    key_format: CacheKeyFormat,
    serializer: S,
    compressor: C,
    label: BackendLabel,
}

impl<S, C> FeOxDbBackend<S, C>
where
    S: Format,
    C: Compressor,
{
    /// Forces pending writes to disk.
    ///
    /// FeOxDB buffers writes in memory and flushes them periodically (~100ms).
    /// Call this when you need to ensure data is persisted before proceeding,
    /// or in tests to verify disk behavior synchronously.
    ///
    /// No-op in memory-only mode.
    pub fn flush(&self) {
        self.store.flush();
    }
}

impl FeOxDbBackend<JsonFormat, PassthroughCompressor> {
    /// Starts building a new backend.
    pub fn builder() -> FeOxDbBackendBuilder<JsonFormat, PassthroughCompressor> {
        FeOxDbBackendBuilder::default()
    }

    /// In-memory backend for tests.
    ///
    /// Data is lost when dropped. Equivalent to `builder().build()`.
    ///
    /// ```
    /// use hitbox_feoxdb::FeOxDbBackend;
    ///
    /// let backend = FeOxDbBackend::in_memory()
    ///     .expect("Failed to create in-memory backend");
    /// ```
    pub fn in_memory() -> Result<Self, FeOxDbError> {
        let store = FeoxStore::builder().enable_ttl(true).build()?;

        Ok(Self {
            store: Arc::new(store),
            key_format: CacheKeyFormat::Bitcode,
            serializer: JsonFormat,
            compressor: PassthroughCompressor,
            label: BackendLabel::new_static("feoxdb"),
        })
    }
}

/// Builder for [`FeOxDbBackend`].
///
/// ```no_run
/// use hitbox_feoxdb::FeOxDbBackend;
/// use hitbox_backend::format::BincodeFormat;
///
/// let backend = FeOxDbBackend::builder()
///     .path("/var/cache/myapp")
///     .max_file_size(5 * 1024 * 1024 * 1024)  // 5 GB
///     .max_memory(256 * 1024 * 1024)          // 256 MB
///     .value_format(BincodeFormat)
///     .build()?;
/// # Ok::<(), hitbox_feoxdb::FeOxDbError>(())
/// ```
pub struct FeOxDbBackendBuilder<S = JsonFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    path: Option<PathBuf>,
    max_file_size: Option<u64>,
    max_memory: Option<usize>,
    key_format: CacheKeyFormat,
    serializer: S,
    compressor: C,
    label: BackendLabel,
}

impl Default for FeOxDbBackendBuilder<JsonFormat, PassthroughCompressor> {
    fn default() -> Self {
        Self {
            path: None,
            max_file_size: None,
            max_memory: None,
            key_format: CacheKeyFormat::Bitcode,
            serializer: JsonFormat,
            compressor: PassthroughCompressor,
            label: BackendLabel::new_static("feoxdb"),
        }
    }
}

impl<S, C> FeOxDbBackendBuilder<S, C>
where
    S: Format,
    C: Compressor,
{
    /// Enables persistent storage at the given path.
    ///
    /// Without this, data lives only in memory and is lost on restart.
    /// If path is a directory, creates `cache.db` inside it.
    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Pre-allocates disk space and caps maximum storage.
    ///
    /// The file is allocated upfront to avoid fragmentation. Writes fail with
    /// `OutOfSpace` when full. Ignored in memory-only mode.
    ///
    /// Default: 1 GB
    pub fn max_file_size(mut self, bytes: u64) -> Self {
        self.max_file_size = Some(bytes);
        self
    }

    /// Limits RAM usage.
    ///
    /// In memory-only mode, this is your total cache capacity.
    /// In persistent mode, this limits the read cache for disk data.
    ///
    /// Unlike Moka, FeOxDB has no automatic eviction — writes fail with
    /// `OutOfMemory` when the limit is reached.
    ///
    /// Default: 1 GB
    pub fn max_memory(mut self, bytes: usize) -> Self {
        self.max_memory = Some(bytes);
        self
    }

    /// Cache key serialization format. Rarely needs changing.
    pub fn key_format(mut self, format: CacheKeyFormat) -> Self {
        self.key_format = format;
        self
    }

    /// Identifies this backend in multi-tier setups and metrics.
    pub fn label(mut self, label: impl Into<BackendLabel>) -> Self {
        self.label = label.into();
        self
    }

    /// Value serialization format.
    ///
    /// `BincodeFormat` is a good default for production — fast and compact.
    /// `JsonFormat` (default) is useful for debugging since values are readable.
    pub fn value_format<NewS>(self, serializer: NewS) -> FeOxDbBackendBuilder<NewS, C>
    where
        NewS: Format,
    {
        FeOxDbBackendBuilder {
            path: self.path,
            max_file_size: self.max_file_size,
            max_memory: self.max_memory,
            key_format: self.key_format,
            serializer,
            compressor: self.compressor,
            label: self.label,
        }
    }

    /// Compression for cached values.
    ///
    /// For disk-based caches, compression often improves performance by
    /// reducing I/O, even accounting for CPU overhead. `ZstdCompressor`
    /// offers the best ratio with good speed.
    pub fn compressor<NewC>(self, compressor: NewC) -> FeOxDbBackendBuilder<S, NewC>
    where
        NewC: Compressor,
    {
        FeOxDbBackendBuilder {
            path: self.path,
            max_file_size: self.max_file_size,
            max_memory: self.max_memory,
            key_format: self.key_format,
            serializer: self.serializer,
            compressor,
            label: self.label,
        }
    }

    /// Creates the backend.
    ///
    /// Fails if the database file can't be opened or created.
    pub fn build(self) -> Result<FeOxDbBackend<S, C>, FeOxDbError> {
        let mut builder = FeoxStore::builder().enable_ttl(true);

        if let Some(mut path) = self.path {
            if path.is_dir() {
                path.push("cache.db");
            }
            let path_str = path.to_string_lossy().to_string();
            builder = builder.device_path(path_str);
        }

        if let Some(file_size) = self.max_file_size {
            builder = builder.file_size(file_size);
        }

        if let Some(memory) = self.max_memory {
            builder = builder.max_memory(memory);
        }

        let store = builder.build()?;

        Ok(FeOxDbBackend {
            store: Arc::new(store),
            key_format: self.key_format,
            serializer: self.serializer,
            compressor: self.compressor,
            label: self.label,
        })
    }
}

#[async_trait]
impl<S, C> Backend for FeOxDbBackend<S, C>
where
    S: Format + Send + Sync,
    C: Compressor + Send + Sync,
{
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        let store = self.store.clone();

        let key_bytes = encode_to_vec(key, bincode_config())
            .map_err(|e| BackendError::InternalError(Box::new(e)))?;

        tokio::task::spawn_blocking(move || match store.get(&key_bytes) {
            Ok(encoded) => {
                let (serializable, _): (SerializableCacheValue, _) =
                    decode_from_slice(&encoded, bincode_config())
                        .map_err(|e| BackendError::InternalError(Box::new(e)))?;

                let cache_value: CacheValue<Raw> = serializable.into();

                if let Some(expire_time) = cache_value.expire()
                    && expire_time < Utc::now()
                {
                    return Ok(None);
                }

                Ok(Some(cache_value))
            }
            Err(FeoxError::KeyNotFound) => Ok(None),
            Err(e) => Err(BackendError::InternalError(Box::new(e))),
        })
        .await
        .map_err(|e| BackendError::InternalError(Box::new(e)))?
    }

    async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
        let store = self.store.clone();

        let key_bytes = encode_to_vec(key, bincode_config())
            .map_err(|e| BackendError::InternalError(Box::new(e)))?;

        // Compute TTL from value.ttl() (derived from value.expire)
        let ttl = value.ttl();

        let serializable: SerializableCacheValue = value.into();
        let value_bytes = encode_to_vec(&serializable, bincode_config())
            .map_err(|e| BackendError::InternalError(Box::new(e)))?;

        tokio::task::spawn_blocking(move || {
            ttl.map(|ttl_duration| ttl_duration.as_secs())
                .map(|ttl_secs| store.insert_with_ttl(&key_bytes, &value_bytes, ttl_secs))
                .unwrap_or_else(|| store.insert(&key_bytes, &value_bytes))
                .map_err(|e| BackendError::InternalError(Box::new(e)))?;
            Ok(())
        })
        .await
        .map_err(|e| BackendError::InternalError(Box::new(e)))?
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let store = self.store.clone();

        let key_bytes = encode_to_vec(key, bincode_config())
            .map_err(|e| BackendError::InternalError(Box::new(e)))?;

        tokio::task::spawn_blocking(move || {
            match store.delete(&key_bytes) {
                Ok(()) => Ok(DeleteStatus::Deleted(1)),
                Err(FeoxError::KeyNotFound) => Ok(DeleteStatus::Missing),
                Err(e) => Err(BackendError::InternalError(Box::new(e))),
            }
        })
        .await
        .map_err(|e| BackendError::InternalError(Box::new(e)))?
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

    fn label(&self) -> BackendLabel {
        self.label.clone()
    }
}

// Explicit CacheBackend implementation using default trait methods
impl<S, C> hitbox_backend::CacheBackend for FeOxDbBackend<S, C>
where
    S: Format + Send + Sync,
    C: Compressor + Send + Sync,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::builder()
            .path(temp_dir.path())
            .build()
            .unwrap();

        let key = CacheKey::from_str("test-key", "1");
        let value = CacheValue::new(
            Bytes::from(&b"test-value"[..]),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        // Write with 1 hour TTL
        backend.write(&key, value.clone()).await.unwrap();

        // Read
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data().as_ref(), b"test-value");
    }

    #[tokio::test]
    async fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::builder()
            .path(temp_dir.path())
            .build()
            .unwrap();

        let key = CacheKey::from_str("delete-key", "1");
        let value = CacheValue::new(
            Bytes::from(&b"test-value"[..]),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        // Write
        backend.write(&key, value).await.unwrap();

        // Delete
        let status = backend.remove(&key).await.unwrap();
        assert_eq!(status, DeleteStatus::Deleted(1));

        // Verify deleted
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_missing() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::builder()
            .path(temp_dir.path())
            .build()
            .unwrap();

        let key = CacheKey::from_str("nonexistent", "1");
        let status = backend.remove(&key).await.unwrap();
        assert_eq!(status, DeleteStatus::Missing);
    }

    #[tokio::test]
    async fn test_read_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::builder()
            .path(temp_dir.path())
            .build()
            .unwrap();

        let key = CacheKey::from_str("nonexistent-read", "1");
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_backend() {
        let backend = FeOxDbBackend::in_memory().unwrap();

        let key = CacheKey::from_str("memory-key", "1");
        let value = CacheValue::new(
            Bytes::from(&b"memory-value"[..]),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        // Write
        backend.write(&key, value).await.unwrap();

        // Read
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data().as_ref(), b"memory-value");
    }

    #[tokio::test]
    async fn test_clone_shares_store() {
        let temp_dir = TempDir::new().unwrap();
        let backend1 = FeOxDbBackend::builder()
            .path(temp_dir.path())
            .build()
            .unwrap();
        let backend2 = backend1.clone();

        let key = CacheKey::from_str("shared-key", "1");
        let value = CacheValue::new(
            Bytes::from(&b"shared-value"[..]),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        // Write with backend1
        backend1.write(&key, value).await.unwrap();

        // Read with backend2
        let result = backend2.read(&key).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data().as_ref(), b"shared-value");
    }

    #[tokio::test]
    async fn test_per_key_ttl() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::builder()
            .path(temp_dir.path())
            .build()
            .unwrap();

        let now = Utc::now();
        let expire_1h = now + chrono::Duration::hours(1);
        let expire_24h = now + chrono::Duration::hours(24);

        // Key 1 with 1 hour TTL
        let key1 = CacheKey::from_str("key1", "1");
        let value1 = CacheValue::new(Bytes::from(&b"value1"[..]), Some(expire_1h), None);
        backend.write(&key1, value1).await.unwrap();

        // Key 2 with 24 hour TTL
        let key2 = CacheKey::from_str("key2", "1");
        let value2 = CacheValue::new(Bytes::from(&b"value2"[..]), Some(expire_24h), None);
        backend.write(&key2, value2).await.unwrap();

        // Read and verify TTLs are preserved
        let read1 = backend
            .read(&key1)
            .await
            .unwrap()
            .expect("key1 should exist");
        let read2 = backend
            .read(&key2)
            .await
            .unwrap()
            .expect("key2 should exist");

        // Expire times should be approximately equal (within 1 second tolerance)
        let tolerance = chrono::Duration::seconds(1);
        assert!(
            (read1.expire().unwrap() - expire_1h).abs() < tolerance,
            "key1 expire time should be ~1 hour from now"
        );
        assert!(
            (read2.expire().unwrap() - expire_24h).abs() < tolerance,
            "key2 expire time should be ~24 hours from now"
        );
    }

    #[tokio::test]
    async fn test_expired_entry_not_returned() {
        let backend = FeOxDbBackend::in_memory().unwrap();

        // Write entry that's already expired
        let key = CacheKey::from_str("expired-key", "1");
        let expired_time = Utc::now() - chrono::Duration::seconds(10);
        let value = CacheValue::new(Bytes::from(&b"expired"[..]), Some(expired_time), None);
        backend.write(&key, value).await.unwrap();

        // Should not be returned (filtered by expire check)
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_none(), "Expired entry should not be returned");
    }

    #[tokio::test]
    async fn test_memory_limit_exceeded() {
        // Very small memory limit
        let backend = FeOxDbBackend::builder()
            .max_memory(1024) // 1 KB
            .build()
            .unwrap();

        // Try to write data larger than the limit
        let key = CacheKey::from_str("big-key", "1");
        let large_data = vec![0u8; 2048]; // 2 KB
        let value = CacheValue::new(
            Bytes::from(large_data),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        let result = backend.write(&key, value).await;
        assert!(
            result.is_err(),
            "Write should fail when exceeding memory limit"
        );
    }

    #[tokio::test]
    async fn test_builder_with_label() {
        let backend = FeOxDbBackend::builder()
            .label("custom-label")
            .build()
            .unwrap();

        assert_eq!(backend.label().as_ref(), "custom-label");
    }

    #[tokio::test]
    async fn test_builder_with_custom_format() {
        use hitbox_backend::format::BincodeFormat;

        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::builder()
            .path(temp_dir.path())
            .value_format(BincodeFormat)
            .build()
            .unwrap();

        // Write and read to verify format works
        let key = CacheKey::from_str("format-key", "1");
        let value = CacheValue::new(
            Bytes::from(&b"format-value"[..]),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        backend.write(&key, value).await.unwrap();
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data().as_ref(), b"format-value");
    }

    #[tokio::test]
    async fn test_flush_persists_data() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("cache.db");

        // Write and flush
        {
            let backend = FeOxDbBackend::builder()
                .path(temp_dir.path())
                .build()
                .unwrap();

            let key = CacheKey::from_str("persist-key", "1");
            let value = CacheValue::new(
                Bytes::from(&b"persist-value"[..]),
                Some(Utc::now() + chrono::Duration::hours(1)),
                None,
            );
            backend.write(&key, value).await.unwrap();
            backend.flush();
        }

        // Reopen and verify data persisted
        let backend = FeOxDbBackend::builder().path(&db_path).build().unwrap();

        let key = CacheKey::from_str("persist-key", "1");
        let result = backend.read(&key).await.unwrap();
        assert!(
            result.is_some(),
            "Data should persist after flush and reopen"
        );
        assert_eq!(result.unwrap().data().as_ref(), b"persist-value");
    }

    #[tokio::test]
    async fn test_file_size_limit_drops_excess_writes() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("cache.db");

        let file_size_limit = 10 * 1024 * 1024; // 10 MB
        let chunk_size = 256 * 1024; // 256 KB chunks
        let num_chunks = 60; // ~15 MB total - exceeds 10 MB limit

        // Write more data than the file size limit allows
        {
            let backend = FeOxDbBackend::builder()
                .path(temp_dir.path())
                .max_file_size(file_size_limit)
                .build()
                .unwrap();

            let chunk = vec![0u8; chunk_size];
            for i in 0..num_chunks {
                let key = CacheKey::from_str(&format!("chunk-{}", i), "1");
                let value = CacheValue::new(
                    Bytes::from(chunk.clone()),
                    Some(Utc::now() + chrono::Duration::hours(1)),
                    None,
                );
                let _ = backend.write(&key, value).await;
                // Periodic flush to persist data incrementally
                if i % 5 == 4 {
                    backend.flush();
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
            backend.flush();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // Reopen and count how many chunks actually persisted
        let backend = FeOxDbBackend::builder()
            .path(&db_path)
            .max_file_size(file_size_limit)
            .build()
            .unwrap();

        let mut persisted_count = 0;
        for i in 0..num_chunks {
            let key = CacheKey::from_str(&format!("chunk-{}", i), "1");
            if backend.read(&key).await.unwrap().is_some() {
                persisted_count += 1;
            }
        }

        // Some writes should persist, but not all (disk fills up)
        assert!(persisted_count > 0, "At least some chunks should persist");
        assert!(
            persisted_count < num_chunks,
            "Not all chunks should persist when exceeding file size limit. \
             Persisted {}/{} chunks",
            persisted_count,
            num_chunks
        );
    }
}
