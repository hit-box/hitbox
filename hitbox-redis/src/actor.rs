//! Redis backend actor implementation.
use crate::error::Error;
use async_trait::async_trait;
use hitbox_backend::{
    serializer::{JsonSerializer, Serializer},
    Backend, BackendError, BackendResult, CacheBackend, CacheableResponse, CachedValue, Delete,
    DeleteStatus, Get, Lock, LockStatus, Set,
};
use log::{debug, info};
use redis::{aio::ConnectionManager, Client};

/// Redis cache backend based on redis-rs crate.
///
/// This actor provides redis as storage [Backend] for hitbox.
/// Its use one [MultiplexedConnection] for asynchronous network interaction.
///
/// [MultiplexedConnection]: redis::aio::MultiplexedConnection
/// [Backend]: hitbox_backend::Backend
pub struct RedisBackend {
    connection: ConnectionManager,
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
    ///     let backend = RedisBackend::new().await;
    /// }
    /// ```
    pub async fn new() -> Result<RedisBackend, Error> {
        Self::builder().build().await
    }

    /// Creates new RedisBackend builder with default settings.
    pub fn builder() -> RedisBackendBuilder {
        RedisBackendBuilder::default()
    }
}

/// Part of builder pattern implemetation for RedisBackend actor.
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
    pub async fn build(&self) -> Result<RedisBackend, Error> {
        let client = Client::open(self.connection_info.as_str())?;
        let connection = client.get_tokio_connection_manager().await?;
        Ok(RedisBackend { connection })
    }
}

// /// Implementation of Actix Handler for Lock message.
// impl Handler<Lock> for RedisBackend {
    // type Result = ResponseFuture<Result<LockStatus, BackendError>>;

    // fn handle(&mut self, msg: Lock, _: &mut Self::Context) -> Self::Result {
        // debug!("Redis Lock: {}", msg.key);
        // let mut con = self.connection.clone();
        // Box::pin(async move {
            // redis::cmd("SET")
                // .arg(format!("lock::{}", msg.key))
                // .arg("")
                // .arg("NX")
                // .arg("EX")
                // .arg(msg.ttl)
                // .query_async(&mut con)
                // .await
                // .map(|res: Option<String>| -> LockStatus {
                    // if res.is_some() {
                        // LockStatus::Acquired
                    // } else {
                        // LockStatus::Locked
                    // }
                // })
                // .map_err(Error::from)
                // .map_err(BackendError::from)
        // })
    // }
// }

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get<T>(&self, key: String) -> BackendResult<CachedValue<T>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        let mut con = self.connection.clone();
        let result: Vec<u8> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut con)
            .await
            .map_err(Error::from)
            .map_err(BackendError::from)?;
        JsonSerializer::<Vec<u8>>::deserialize(result).map_err(BackendError::from)
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        let mut con = self.connection.clone();
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

    async fn set<T: Sync>(
        &self,
        key: String,
        value: &CachedValue<T>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        let mut con = self.connection.clone();
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
}
