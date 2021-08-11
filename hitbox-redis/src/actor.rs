//! Redis backend actor implementation.
use crate::error::Error;
use actix::prelude::*;
use hitbox_backend::{Backend, BackendError, Delete, DeleteStatus, Get, Lock, LockStatus, Set};
use log::{debug, info};
use redis::aio::ConnectionLike;
use redis::IntoConnectionInfo;
#[cfg(feature = "single")]
use redis::{aio::ConnectionManager, Client};
#[cfg(feature = "cluster")]
use redis_cluster_async::{Client as ClusterClient, Connection as ClusterConnection};
use std::marker::Unpin;

/// United builder for [RedisSingleBackend] and [RedisClusterBackend]
///
/// [RedisSingleBackend]: RedisSingleBackend
/// [RedisClusterBackend]: RedisClusterBackend
pub struct RedisBuilder;

impl RedisBuilder {
    /// Create new backend for redis single instance
    ///
    /// # Examples
    /// ```
    /// use hitbox_redis::RedisBuilder;
    ///
    /// #[actix_rt::main]
    /// async fn main() {
    ///     let backend = RedisBuilder::single("redis://127.0.0.1:6379").await;
    /// }
    /// ```
    #[cfg(feature = "single")]
    pub async fn single<T: IntoConnectionInfo>(
        address: T,
    ) -> Result<RedisBackend<RedisSingleBackend>, Error> {
        RedisSingleBackend::build(address).await
    }
    /// Create new backend for redis cluster instance
    ///
    /// # Examples
    /// ```
    /// use hitbox_redis::RedisBuilder;
    ///
    /// #[actix_rt::main]
    /// async fn main() {
    ///     let nodes = vec![
    ///         "redis://127.0.0.1:6379",
    ///         "redis://127.0.0.1:6378",
    ///         "redis://127.0.0.1:6377",
    ///     ];
    ///     let readonly = true;
    ///     let backend = RedisBuilder::cluster(nodes, readonly).await;
    /// }
    /// ```
    ///
    #[cfg(feature = "cluster")]
    pub async fn cluster<T: IntoConnectionInfo + Clone>(
        addresses: Vec<T>,
        readonly: bool,
    ) -> Result<RedisBackend<RedisClusterBackend>, Error> {
        RedisClusterBackend::build(addresses, readonly).await
    }
}
/// Redis cache backend based on redis-rs crate.
///
/// This actor provides redis as storage [Backend] for hitbox.
/// Its use one [MultiplexedConnection] for asynchronous network interaction.
///
/// [MultiplexedConnection]: redis::aio::MultiplexedConnection
/// [Backend]: hitbox_backend::Backend
pub struct RedisBackend<B>
where
    B: RedisConnection,
{
    backend: B,
}

/// Trait describes interaction with cache [Backend] and redis client.
///
/// Main idea of this trait is creating an actor from different redis clients
/// [Backend]: hitbox_backend::Backend
pub trait RedisConnection
where
    Self::Connection: ConnectionLike,
{
    /// Associated type describes the type of connection.
    type Connection;

    /// Get connection to the master node.
    fn get_master_connection(&self) -> Self::Connection;

    /// Get connection to the slave node.
    fn get_slave_connection(&self) -> Self::Connection;
}

/// Redis single backend based redis-rs crate.
///
/// This backend create connections to redis single instance and create actor [RedisBackend],
/// which provides redis as storage [Backend] for hitbox.
/// Its use one [MultiplexedConnection] for asynchronous network interaction.
///
/// [MultiplexedConnection]: redis::aio::MultiplexedConnection
/// [Backend]: hitbox_backend::Backend
/// [RedisBackend]: RedisBackend
#[cfg(feature = "single")]
pub struct RedisSingleBackend {
    master: ConnectionManager,
    slave: ConnectionManager,
}

#[cfg(feature = "single")]
impl RedisSingleBackend {
    ///
    pub async fn build<T: IntoConnectionInfo>(
        connection_info: T,
    ) -> Result<RedisBackend<Self>, Error> {
        let client = Client::open(connection_info)?;
        let connection = client.get_tokio_connection_manager().await?;
        let backend = Self {
            master: connection.clone(),
            slave: connection,
        };
        Ok(RedisBackend { backend })
    }
}

#[cfg(feature = "single")]
impl RedisConnection for RedisSingleBackend {
    type Connection = ConnectionManager;

    fn get_master_connection(&self) -> Self::Connection {
        self.master.clone()
    }

    fn get_slave_connection(&self) -> Self::Connection {
        self.slave.clone()
    }
}

/// Redis cluster backend based on redis_cluster_async crate.
/// redis_cluster_async based on redis-rs and gives similar functionality
/// for working with reids.
///
/// This backend create connections to redis cluster instance and
/// create actor [RedisBackend], which provides redis as storage [Backend] for hitbox.
/// Its use multiple [MultiplexedConnection] for asynchronous network interaction.
///
/// [MultiplexedConnection]: redis::aio::MultiplexedConnection
/// [Backend]: hitbox_backend::Backend
/// [RedisBackend]: RedisBackend
#[cfg(feature = "cluster")]
pub struct RedisClusterBackend {
    master: ClusterConnection,
    slave: ClusterConnection,
}

#[cfg(feature = "cluster")]
impl RedisClusterBackend {
    /// Create new backend instance for redis cluster instance with default settings.
    pub async fn build<T: IntoConnectionInfo + Clone>(
        addresses: Vec<T>,
        readonly: bool,
    ) -> Result<RedisBackend<Self>, Error> {
        let master_client = ClusterClient::open(addresses.clone())?;
        let slave_client = ClusterClient::open(addresses)?.readonly(readonly);
        let master_connection = master_client.get_connection().await?;
        let slave_connection = slave_client.get_connection().await?;
        let backend = Self {
            master: master_connection,
            slave: slave_connection,
        };
        Ok(RedisBackend { backend })
    }
}

#[cfg(feature = "cluster")]
impl RedisConnection for RedisClusterBackend {
    type Connection = ClusterConnection;

    fn get_master_connection(&self) -> Self::Connection {
        self.master.clone()
    }

    fn get_slave_connection(&self) -> Self::Connection {
        self.slave.clone()
    }
}

impl<B> Backend for RedisBackend<B>
where
    B: RedisConnection + Unpin + 'static,
{
    type Actor = Self;
    type Context = Context<Self>;
}

/// Implementation actix Actor trait for Redis cache backend.
impl<B> Actor for RedisBackend<B>
where
    B: RedisConnection + Unpin + 'static,
{
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
    }
}

impl<B> Handler<Get> for RedisBackend<B>
where
    B: RedisConnection + Unpin + 'static,
{
    type Result = ResponseFuture<Result<Option<Vec<u8>>, BackendError>>;

    fn handle(&mut self, msg: Get, _: &mut Self::Context) -> Self::Result {
        let mut con = self.backend.get_slave_connection();
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

impl<B> Handler<Set> for RedisBackend<B>
where
    B: RedisConnection + Unpin + 'static,
{
    type Result = ResponseFuture<Result<String, BackendError>>;

    fn handle(&mut self, msg: Set, _: &mut Self::Context) -> Self::Result {
        let mut con = self.backend.get_master_connection();
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

impl<B> Handler<Delete> for RedisBackend<B>
where
    B: RedisConnection + Unpin + 'static,
{
    type Result = ResponseFuture<Result<DeleteStatus, BackendError>>;

    fn handle(&mut self, msg: Delete, _: &mut Self::Context) -> Self::Result {
        let mut con = self.backend.get_master_connection();
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

impl<B> Handler<Lock> for RedisBackend<B>
where
    B: RedisConnection + Unpin + 'static,
{
    type Result = ResponseFuture<Result<LockStatus, BackendError>>;

    fn handle(&mut self, msg: Lock, _: &mut Self::Context) -> Self::Result {
        debug!("Redis Lock: {}", msg.key);
        let mut con = self.backend.get_master_connection();
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
