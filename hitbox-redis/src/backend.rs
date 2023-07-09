//! Redis backend actor implementation.
use crate::error::Error;
use async_trait::async_trait;
use hitbox_backend::{
    serializer::{JsonSerializer, Serializer},
    BackendError, BackendResult, CacheBackend, CacheableResponse, CachedValue, DeleteStatus,
};
use redis::{aio::ConnectionManager, Client};
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
                self.client.get_tokio_connection_manager()
            })
            .await
            .map_err(Error::from)?;
        Ok(manager)
    }
}

/// Part of builder pattern implementation for RedisBackend actor.
pub struct RedisBackendBuilder {
    connection_info: String,
}

impl Default for RedisBackendBuilder {
    fn default() -> Self {
        Self {
            connection_info: "redis://127.0.0.1/".to_owned(),
        }
    }
}

impl RedisBackendBuilder {
    /// Set connection info (host, port, database, etc.) for RedisBackend actor.
    pub fn server(mut self, connection_info: String) -> Self {
        self.connection_info = connection_info;
        self
    }

    /// Create new instance of Redis backend with passed settings.
    pub fn build(self) -> Result<RedisBackend, Error> {
        Ok(RedisBackend {
            client: Client::open(self.connection_info)?,
            connection: OnceCell::new(),
        })
    }
}

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        let client = self.client.clone();
        async move {
            let mut con = client.get_tokio_connection_manager().await.unwrap();
            let result: Option<Vec<u8>> = redis::cmd("GET")
                .arg(key)
                .query_async(&mut con)
                .await
                .map_err(Error::from)
                .map_err(BackendError::from)?;
            result
                .map(|value| {
                    JsonSerializer::<Vec<u8>>::deserialize(value).map_err(BackendError::from)
                })
                .transpose()
        }
        .await
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        let mut con = self.connection().await?.clone();
        redis::cmd("DEL")
            .arg(key)
            .query_async(&mut con)
            .await
            .map(|res| {
                if res > 0 {
                    DeleteStatus::Deleted(res)
                } else {
                    DeleteStatus::Missing
                }
            })
            .map_err(Error::from)
            .map_err(BackendError::from)
    }

    async fn set<T>(
        &self,
        key: String,
        value: &CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        T::Cached: serde::Serialize + Send + Sync,
    {
        let mut con = self.connection().await?.clone();
        let mut request = redis::cmd("SET");
        let serialized_value =
            JsonSerializer::<Vec<u8>>::serialize(&value).map_err(BackendError::from)?;
        request.arg(key).arg(serialized_value);
        if let Some(ttl) = ttl {
            request.arg("EX").arg(ttl);
        };
        request
            .query_async(&mut con)
            .await
            .map_err(Error::from)
            .map_err(BackendError::from)
    }

    async fn start(&self) -> BackendResult<()> {
        self.connection().await?;
        Ok(())
    }
}
