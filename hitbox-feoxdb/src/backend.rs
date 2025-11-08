use std::{path::Path, sync::Arc, time::Duration};

use async_trait::async_trait;
use bincode::{
    config::standard as bincode_config,
    serde::{decode_from_slice, encode_to_vec},
};
use chrono::{DateTime, Utc};
use feoxdb::{FeoxError, FeoxStore};
use hitbox_backend::serializer::Format;
use hitbox_backend::{Backend, BackendError, BackendResult, CacheKeyFormat, Compressor, DeleteStatus, PassthroughCompressor};
use hitbox_core::{CacheKey, CacheValue};
use serde::{Deserialize, Serialize};

use crate::FeOxDbError;

type Raw = Vec<u8>;

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
            data: value.data,
            stale: value.stale,
            expire: value.expire,
        }
    }
}

impl From<SerializableCacheValue> for CacheValue<Raw> {
    fn from(value: SerializableCacheValue) -> Self {
        CacheValue::new(value.data, value.expire, value.stale)
    }
}

#[derive(Clone)]
pub struct FeOxDbBackend<C: Compressor = PassthroughCompressor> {
    store: Arc<FeoxStore>,
    key_format: CacheKeyFormat,
    value_format: Format,
    compressor: C,
}

impl FeOxDbBackend<PassthroughCompressor> {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, FeOxDbError> {
        let mut path_buf = path.as_ref().to_path_buf();
        if path_buf.is_dir() {
            path_buf.push("cache.db");
        }

        let path_str = path_buf.to_string_lossy().to_string();

        let store = FeoxStore::builder()
            .device_path(path_str)
            .enable_ttl(true)
            .build()?;

        Ok(Self {
            store: Arc::new(store),
            key_format: CacheKeyFormat::Bitcode,
            value_format: Format::Json,
            compressor: PassthroughCompressor,
        })
    }

    pub fn builder() -> FeOxDbBackendBuilder<PassthroughCompressor> {
        FeOxDbBackendBuilder::default()
    }

    pub fn from_store(store: FeoxStore) -> Self {
        Self {
            store: Arc::new(store),
            key_format: CacheKeyFormat::Bitcode,
            value_format: Format::Json,
            compressor: PassthroughCompressor,
        }
    }

    pub fn in_memory() -> Result<Self, FeOxDbError> {
        let store = FeoxStore::builder().enable_ttl(true).build()?;

        Ok(Self {
            store: Arc::new(store),
            key_format: CacheKeyFormat::Bitcode,
            value_format: Format::Json,
            compressor: PassthroughCompressor,
        })
    }
}

pub struct FeOxDbBackendBuilder<C: Compressor = PassthroughCompressor> {
    path: Option<String>,
    key_format: CacheKeyFormat,
    value_format: Format,
    compressor: C,
}

impl Default for FeOxDbBackendBuilder<PassthroughCompressor> {
    fn default() -> Self {
        Self {
            path: None,
            key_format: CacheKeyFormat::Bitcode,
            value_format: Format::Json,
            compressor: PassthroughCompressor,
        }
    }
}

impl<C: Compressor> FeOxDbBackendBuilder<C> {
    pub fn path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }

    pub fn key_format(mut self, format: CacheKeyFormat) -> Self {
        self.key_format = format;
        self
    }

    pub fn value_format(mut self, format: Format) -> Self {
        self.value_format = format;
        self
    }

    pub fn compressor<NewC: Compressor>(self, compressor: NewC) -> FeOxDbBackendBuilder<NewC> {
        FeOxDbBackendBuilder {
            path: self.path,
            key_format: self.key_format,
            value_format: self.value_format,
            compressor,
        }
    }

    pub fn build(self) -> Result<FeOxDbBackend<C>, FeOxDbError> {
        let store = if let Some(path) = self.path {
            let mut path_buf = std::path::PathBuf::from(path);
            if path_buf.is_dir() {
                path_buf.push("cache.db");
            }
            let path_str = path_buf.to_string_lossy().to_string();
            FeoxStore::builder()
                .device_path(path_str)
                .enable_ttl(true)
                .build()?
        } else {
            FeoxStore::builder().enable_ttl(true).build()?
        };

        Ok(FeOxDbBackend {
            store: Arc::new(store),
            key_format: self.key_format,
            value_format: self.value_format,
            compressor: self.compressor,
        })
    }
}

#[async_trait]
impl<C: Compressor + Send + Sync> Backend for FeOxDbBackend<C> {
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

                if let Some(expire_time) = cache_value.expire {
                    if expire_time < Utc::now() {
                        return Ok(None);
                    }
                }

                Ok(Some(cache_value))
            }
            Err(FeoxError::KeyNotFound) => Ok(None),
            Err(e) => Err(BackendError::InternalError(Box::new(e))),
        })
        .await
        .map_err(|e| BackendError::InternalError(Box::new(e)))?
    }

    async fn write(
        &self,
        key: &CacheKey,
        value: CacheValue<Raw>,
        ttl: Option<Duration>,
    ) -> BackendResult<()> {
        let store = self.store.clone();

        let key_bytes = encode_to_vec(key, bincode_config())
            .map_err(|e| BackendError::InternalError(Box::new(e)))?;

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
            let exists = store.contains_key(&key_bytes);

            if exists {
                store
                    .delete(&key_bytes)
                    .map_err(|e| BackendError::InternalError(Box::new(e)))?;
                Ok(DeleteStatus::Deleted(1))
            } else {
                Ok(DeleteStatus::Missing)
            }
        })
        .await
        .map_err(|e| BackendError::InternalError(Box::new(e)))?
    }

    fn value_format(&self) -> &Format {
        &self.value_format
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &self.key_format
    }

    fn compressor(&self) -> &dyn Compressor {
        &self.compressor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::open(temp_dir.path()).unwrap();

        let key = CacheKey::from_str("test-key", "1");
        let value = CacheValue::new(
            b"test-value".to_vec(),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        // Write with 1 hour TTL
        backend
            .write(&key, value.clone(), Some(Duration::from_secs(3600)))
            .await
            .unwrap();

        // Read
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data, b"test-value");
    }

    #[tokio::test]
    async fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::open(temp_dir.path()).unwrap();

        let key = CacheKey::from_str("delete-key", "1");
        let value = CacheValue::new(
            b"test-value".to_vec(),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        // Write
        backend
            .write(&key, value, Some(Duration::from_secs(3600)))
            .await
            .unwrap();

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
        let backend = FeOxDbBackend::open(temp_dir.path()).unwrap();

        let key = CacheKey::from_str("nonexistent", "1");
        let status = backend.remove(&key).await.unwrap();
        assert_eq!(status, DeleteStatus::Missing);
    }

    #[tokio::test]
    async fn test_read_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::open(temp_dir.path()).unwrap();

        let key = CacheKey::from_str("nonexistent-read", "1");
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_backend() {
        let backend = FeOxDbBackend::in_memory().unwrap();

        let key = CacheKey::from_str("memory-key", "1");
        let value = CacheValue::new(
            b"memory-value".to_vec(),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        // Write
        backend
            .write(&key, value, Some(Duration::from_secs(3600)))
            .await
            .unwrap();

        // Read
        let result = backend.read(&key).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data, b"memory-value");
    }

    #[tokio::test]
    async fn test_clone_shares_store() {
        let temp_dir = TempDir::new().unwrap();
        let backend1 = FeOxDbBackend::open(temp_dir.path()).unwrap();
        let backend2 = backend1.clone();

        let key = CacheKey::from_str("shared-key", "1");
        let value = CacheValue::new(
            b"shared-value".to_vec(),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );

        // Write with backend1
        backend1
            .write(&key, value, Some(Duration::from_secs(3600)))
            .await
            .unwrap();

        // Read with backend2
        let result = backend2.read(&key).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().data, b"shared-value");
    }

    #[tokio::test]
    async fn test_per_key_ttl() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FeOxDbBackend::open(temp_dir.path()).unwrap();

        // Key 1 with 1 hour TTL
        let key1 = CacheKey::from_str("key1", "1");
        let value1 = CacheValue::new(
            b"value1".to_vec(),
            Some(Utc::now() + chrono::Duration::hours(1)),
            None,
        );
        backend
            .write(&key1, value1, Some(Duration::from_secs(3600)))
            .await
            .unwrap();

        // Key 2 with 24 hour TTL
        let key2 = CacheKey::from_str("key2", "1");
        let value2 = CacheValue::new(
            b"value2".to_vec(),
            Some(Utc::now() + chrono::Duration::hours(24)),
            None,
        );
        backend
            .write(&key2, value2, Some(Duration::from_secs(86400)))
            .await
            .unwrap();

        // Both should be readable
        assert!(backend.read(&key1).await.unwrap().is_some());
        assert!(backend.read(&key2).await.unwrap().is_some());
    }
}
