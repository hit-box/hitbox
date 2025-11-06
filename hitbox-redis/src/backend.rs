//! Redis backend actor implementation.
use crate::error::Error;
use async_trait::async_trait;
use chrono::Utc;
use hitbox::{CacheKey, CacheValue};
use hitbox_backend::{
    Backend, BackendError, BackendResult, CacheKeyFormat, DeleteStatus,
    serializer::{Format, Raw},
};
use redis::{Client, aio::ConnectionManager};
use tokio::sync::OnceCell;
use tracing::trace;

/// Redis cache backend based on redis-rs crate.
///
/// This struct provides redis as storage [Backend] for hitbox.
/// Its use one [MultiplexedConnection] for asynchronous network interaction.
///
/// [MultiplexedConnection]: redis::aio::MultiplexedConnection
/// [Backend]: hitbox_backend::Backend
#[derive(Clone)]
pub struct RedisBackend {
    client: Client,
    connection: OnceCell<ConnectionManager>,
    format: Format,
    key_format: CacheKeyFormat,
}

impl RedisBackend {
    /// Create new backend instance with default settings.
    ///
    /// # Examples
    /// ```
    /// use hitbox_redis::RedisBackend;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let backend = RedisBackend::new();
    /// }
    /// ```
    pub fn new() -> Result<RedisBackend, BackendError> {
        Ok(Self::builder().build()?)
    }

    /// Creates new RedisBackend builder with default settings.
    pub fn builder() -> RedisBackendBuilder {
        RedisBackendBuilder::default()
    }

    /// Create lazy connection to redis via [ConnectionManager](redis::aio::ConnectionManager)
    pub async fn connection(&self) -> Result<&ConnectionManager, BackendError> {
        trace!("Get connection manager");
        let manager = self
            .connection
            .get_or_try_init(|| {
                trace!("Initialize new redis connection manager");
                self.client.get_connection_manager()
            })
            .await
            .map_err(Error::from)?;
        Ok(manager)
    }
}

/// Part of builder pattern implementation for RedisBackend actor.
pub struct RedisBackendBuilder {
    connection_info: String,
    format: Format,
    key_format: CacheKeyFormat,
}

impl Default for RedisBackendBuilder {
    fn default() -> Self {
        Self {
            connection_info: "redis://127.0.0.1/".to_owned(),
            format: Format::default(),
            key_format: CacheKeyFormat::default(),
        }
    }
}

impl RedisBackendBuilder {
    /// Set connection info (host, port, database, etc.) for RedisBackend actor.
    pub fn server(mut self, connection_info: String) -> Self {
        self.connection_info = connection_info;
        self
    }

    /// Set value serialization format (JSON, Bincode, etc.)
    pub fn value_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Set key serialization format (String, JSON, Bincode, UrlEncoded)
    pub fn key_format(mut self, key_format: CacheKeyFormat) -> Self {
        self.key_format = key_format;
        self
    }

    /// Create new instance of Redis backend with passed settings.
    pub fn build(self) -> Result<RedisBackend, Error> {
        Ok(RedisBackend {
            client: Client::open(self.connection_info)?,
            connection: OnceCell::new(),
            format: self.format,
            key_format: self.key_format,
        })
    }
}

#[async_trait]
impl Backend for RedisBackend {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        let client = self.client.clone();
        let cache_key = self.key_format.serialize(key)?;
        let mut con = client.get_connection_manager().await.map_err(Error::from)?;
        let result: Option<Vec<u8>> = redis::cmd("GET")
            .arg(cache_key)
            .query_async(&mut con)
            .await
            .map_err(Error::from)?;
        Ok(result.map(|value| CacheValue::new(value, Some(Utc::now()), Some(Utc::now()))))
    }

    async fn write(
        &self,
        key: &CacheKey,
        value: CacheValue<Raw>,
        ttl: Option<std::time::Duration>,
    ) -> BackendResult<()> {
        let mut con = self.connection().await?.clone();
        let cache_key = self.key_format.serialize(key)?;

        let mut request = redis::cmd("SET");
        request.arg(cache_key).arg(value.data);

        ttl.map(|ttl_duration| request.arg("EX").arg(ttl_duration.as_secs()));

        request
            .query_async(&mut con)
            .await
            .map_err(Error::from)
            .map_err(BackendError::from)
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let client = self.client.clone();
        let cache_key = self.key_format.serialize(key)?;
        let mut con = client.get_connection_manager().await.map_err(Error::from)?;

        let deleted: i32 = redis::cmd("DEL")
            .arg(cache_key)
            .query_async(&mut con)
            .await
            .map_err(Error::from)?;

        if deleted > 0 {
            Ok(DeleteStatus::Deleted(deleted as u32))
        } else {
            Ok(DeleteStatus::Missing)
        }
    }

    fn value_format(&self) -> &Format {
        &self.format
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &self.key_format
    }

    // async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
    //     let client = self.client.clone();
    //     let cache_key = UrlEncodedKeySerializer::serialize(key)?;
    //     async move {
    //         let mut con = client.get_tokio_connection_manager().await.unwrap();
    //         let result: Option<Vec<u8>> = redis::cmd("GET")
    //             .arg(cache_key)
    //             .query_async(&mut con)
    //             .await
    //             .map_err(Error::from)
    //             .map_err(BackendError::from)?;
    //         result
    //             .map(|value| {
    //                 JsonSerializer::<Vec<u8>>::deserialize(value).map_err(BackendError::from)
    //             })
    //             .transpose()
    //     }
    //     .await
    // }
    //
    // async fn delete(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
    //     let mut con = self.connection().await?.clone();
    //     let cache_key = UrlEncodedKeySerializer::serialize(key)?;
    //     redis::cmd("DEL")
    //         .arg(cache_key)
    //         .query_async(&mut con)
    //         .await
    //         .map(|res| {
    //             if res > 0 {
    //                 DeleteStatus::Deleted(res)
    //             } else {
    //                 DeleteStatus::Missing
    //             }
    //         })
    //         .map_err(Error::from)
    //         .map_err(BackendError::from)
    // }
    //
    // async fn set<T>(
    //     &self,
    //     key: &CacheKey,
    //     value: &CacheValue<T::Cached>,
    //     ttl: Option<u32>,
    // ) -> BackendResult<()>
    // where
    //     T: CacheableResponse + Send,
    //     T::Cached: serde::Serialize + Send + Sync,
    // {
    //     let mut con = self.connection().await?.clone();
    //     let mut request = redis::cmd("SET");
    //     let cache_key = UrlEncodedKeySerializer::serialize(key)?;
    //     let serialized_value =
    //         JsonSerializer::<Vec<u8>>::serialize(value).map_err(BackendError::from)?;
    //     request.arg(cache_key).arg(serialized_value);
    //     if let Some(ttl) = ttl {
    //         request.arg("EX").arg(ttl);
    //     };
    //     request
    //         .query_async(&mut con)
    //         .await
    //         .map_err(Error::from)
    //         .map_err(BackendError::from)
    // }

    // async fn start(&self) -> BackendResult<()> {
    //     self.connection().await?;
    //     Ok(())
    // }
}
