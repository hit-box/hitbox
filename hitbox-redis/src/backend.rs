//! Redis backend actor implementation.
use crate::error::Error;
use async_trait::async_trait;
use chrono::Utc;
use hitbox::{CacheKey, CacheValue};
use hitbox_backend::{
    Backend, BackendError, BackendResult, CacheKeyFormat, Compressor, DeleteStatus,
    PassthroughCompressor,
    serializer::{Format, JsonFormat, Raw},
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
pub struct RedisBackend<S = JsonFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    client: Client,
    connection: OnceCell<ConnectionManager>,
    serializer: S,
    key_format: CacheKeyFormat,
    compressor: C,
}

impl RedisBackend<JsonFormat, PassthroughCompressor> {
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
    pub fn new() -> Result<Self, BackendError> {
        Ok(Self::builder().build()?)
    }

    /// Creates new RedisBackend builder with default settings.
    pub fn builder() -> RedisBackendBuilder<JsonFormat, PassthroughCompressor> {
        RedisBackendBuilder::default()
    }
}

impl<S, C> RedisBackend<S, C>
where
    S: Format,
    C: Compressor,
{
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
pub struct RedisBackendBuilder<S = JsonFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    connection_info: String,
    serializer: S,
    key_format: CacheKeyFormat,
    compressor: C,
}

impl Default for RedisBackendBuilder<JsonFormat, PassthroughCompressor> {
    fn default() -> Self {
        Self {
            connection_info: "redis://127.0.0.1/".to_owned(),
            serializer: JsonFormat,
            key_format: CacheKeyFormat::default(),
            compressor: PassthroughCompressor,
        }
    }
}

impl<S, C> RedisBackendBuilder<S, C>
where
    S: Format,
    C: Compressor,
{
    /// Set connection info (host, port, database, etc.) for RedisBackend actor.
    pub fn server(mut self, connection_info: String) -> Self {
        self.connection_info = connection_info;
        self
    }

    /// Set value serialization format (JSON, Bincode, etc.)
    pub fn value_format<NewS>(self, serializer: NewS) -> RedisBackendBuilder<NewS, C>
    where
        NewS: Format,
    {
        RedisBackendBuilder {
            connection_info: self.connection_info,
            serializer,
            key_format: self.key_format,
            compressor: self.compressor,
        }
    }

    /// Set key serialization format (String, JSON, Bincode, UrlEncoded)
    pub fn key_format(mut self, key_format: CacheKeyFormat) -> Self {
        self.key_format = key_format;
        self
    }

    /// Set compressor for value compression
    pub fn compressor<NewC>(self, compressor: NewC) -> RedisBackendBuilder<S, NewC>
    where
        NewC: Compressor,
    {
        RedisBackendBuilder {
            connection_info: self.connection_info,
            serializer: self.serializer,
            key_format: self.key_format,
            compressor,
        }
    }

    /// Create new instance of Redis backend with passed settings.
    pub fn build(self) -> Result<RedisBackend<S, C>, Error> {
        Ok(RedisBackend {
            client: Client::open(self.connection_info)?,
            connection: OnceCell::new(),
            serializer: self.serializer,
            key_format: self.key_format,
            compressor: self.compressor,
        })
    }
}

#[async_trait]
impl<S, C> Backend for RedisBackend<S, C>
where
    S: Format + Send + Sync,
    C: Compressor + Send + Sync,
{
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

    fn value_format(&self) -> &dyn Format {
        &self.serializer
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &self.key_format
    }

    fn compressor(&self) -> &dyn Compressor {
        &self.compressor
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
    //                 JsonFormat::<Vec<u8>>::deserialize(value).map_err(BackendError::from)
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
    //         JsonFormat::<Vec<u8>>::serialize(value).map_err(BackendError::from)?;
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
