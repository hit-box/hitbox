//! Redis backend actor implementation.
use crate::error::Error;
use actix::prelude::*;
use actix_cache_backend::{
    Backend, BackendError, Delete, DeleteStatus, Get, Lock, LockStatus, Set,
};
use log::{debug, info};
use redis::{aio::MultiplexedConnection, Client};

/// Redis cache backend based on redis-rs crate.
///
/// This actor provides redis as storage [Backend] for actix-cache.
/// Its use one [MultiplexedConnection] for asynchronous network interaction.
///
/// [MultiplexedConnection]: TODO
/// [Backend]: /actix_cache_backend/trait.Backend.html
pub struct RedisBackend {
    connection: MultiplexedConnection,
}

impl RedisBackend {
    /// Create new backend instance with default settings.
    ///
    /// # Examples
    /// ```
    /// use actix_cache_redis::RedisBackend;
    ///
    /// #[actix_rt::main]
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
        let connection = client.get_multiplexed_tokio_connection().await?;
        Ok(RedisBackend{ connection })
    }
}

impl Backend for RedisBackend {
    type Actor = Self;
    type Context = Context<Self>;
}

/// Implementation actix Actor trait for Redis cache backend.
impl Actor for RedisBackend {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
    }
}

/// Implementation of Actix Handler for Get message.
impl Handler<Get> for RedisBackend {
    type Result = ResponseFuture<Result<Option<String>, BackendError>>;

    fn handle(&mut self, msg: Get, _: &mut Self::Context) -> Self::Result {
        let mut con = self.connection.clone();
        let fut = async move {
            redis::cmd("GET")
                .arg(msg.key)
                .query_async(&mut con)
                .await
                .map_err(Error::from)
                .map_err(BackendError::from)
        };
        Box::pin(fut)
    }
}

/// Implementation of Actix Handler for Set message.
impl Handler<Set> for RedisBackend {
    type Result = ResponseFuture<Result<String, BackendError>>;

    fn handle(&mut self, msg: Set, _: &mut Self::Context) -> Self::Result {
        let mut con = self.connection.clone();
        Box::pin(async move {
            let mut request = redis::cmd("SET");
            request.arg(msg.key).arg(msg.value);
            if let Some(ttl) = msg.ttl {
                request.arg("EX").arg(ttl);
            };
            request
                .query_async(&mut con)
                .await
                .map_err(Error::from)
                .map_err(BackendError::from)
        })
    }
}

/// Implementation of Actix Handler for Delete message.
impl Handler<Delete> for RedisBackend {
    type Result = ResponseFuture<Result<DeleteStatus, BackendError>>;

    fn handle(&mut self, msg: Delete, _: &mut Self::Context) -> Self::Result {
        let mut con = self.connection.clone();
        Box::pin(async move {
            redis::cmd("DEL")
                .arg(msg.key)
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
        })
    }
}

/// Implementation of Actix Handler for Lock message.
impl Handler<Lock> for RedisBackend {
    type Result = ResponseFuture<Result<LockStatus, BackendError>>;

    fn handle(&mut self, msg: Lock, _: &mut Self::Context) -> Self::Result {
        debug!("Redis Lock: {}", msg.key);
        let mut con = self.connection.clone();
        Box::pin(async move {
            redis::cmd("SET")
                .arg(format!("lock::{}", msg.key))
                .arg("")
                .arg("NX")
                .arg("EX")
                .arg(msg.ttl)
                .query_async(&mut con)
                .await
                .map(|res: Option<String>| -> LockStatus {
                    if res.is_some() {
                        LockStatus::Acquired
                    } else {
                        LockStatus::Locked
                    }
                })
                .map_err(Error::from)
                .map_err(BackendError::from)
        })
    }
}
