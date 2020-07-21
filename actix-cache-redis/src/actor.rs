use crate::error::Error;
use actix::prelude::*;
use actix_cache_backend::{BackendError, Get, Set, Backend};
use log::{debug, info};
use redis::{aio::MultiplexedConnection, Client};

/// Actix Redis cache backend actor.
pub struct RedisActor {
    #[allow(dead_code)]
    client: Client,
    connection: MultiplexedConnection,
}

impl RedisActor {
    pub async fn new() -> Result<RedisActor, Error> {
        Self::builder().build().await
    }
    pub fn builder() -> RedisActorBuilder {
        RedisActorBuilder::default()
    }
}

pub struct RedisActorBuilder {
    connection_info: String,
}

impl Default for RedisActorBuilder {
    fn default() -> Self {
        Self {
            connection_info: "redis://127.0.0.1/".to_owned(),
        }
    }
}

impl RedisActorBuilder {
    pub async fn build(&self) -> Result<RedisActor, Error> {
        let client = Client::open(self.connection_info.as_str())?;
        let connection = client.get_multiplexed_tokio_connection().await?;
        Ok(RedisActor { client, connection })
    }
}

impl Backend for RedisActor 
{
    type Actor = Self;
    type Context = Context<Self>;
}

/// Implementation actix Actor trait for Redis cache backend.
impl Actor for RedisActor {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
    }
}

/// Implementation of Actix Handler for Get message.
impl Handler<Get> for RedisActor {
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

///// Actix message implements writing Redis value by key.
// #[derive(Message, Debug, Clone)]
// #[rtype(result = "Result<String, Error>")]
// pub struct Set {
    // pub key: String,
    // pub value: String,
    // pub ttl: Option<u32>,
// }

/// Implementation of Actix Handler for Set message.
impl Handler<Set> for RedisActor {
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
                .query_async(&mut con).
                await
                .map_err(Error::from)
                .map_err(BackendError::from)
        })
    }
}

/// Status of deleting result.
#[derive(Debug, PartialEq)]
pub enum DeleteStatus {
    /// Record sucessfully deleted.
    Deleted(u32),
    /// Record already missing.
    Missing,
}

/// Struct represent deleting record message.
#[derive(Message, Debug)]
#[rtype(result = "Result<DeleteStatus, Error>")]
pub struct Delete {
    pub key: String,
}

/// Implementation of Actix Handler for Delete message.
impl Handler<Delete> for RedisActor {
    type Result = ResponseFuture<Result<DeleteStatus, Error>>;

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
        })
    }
}

/// Struct represent locking process.
#[derive(Message, Debug, Clone)]
#[rtype(result = "Result<LockStatus, Error>")]
pub struct Lock {
    pub key: String,
    pub ttl: u32,
}

/// Enum for representing status of Lock object in redis.
#[derive(Debug, PartialEq)]
pub enum LockStatus {
    /// Lock sucsesfully created and acquired.
    Acquired,
    /// Lock object already acquired (locked).
    Locked,
}

/// Implementation of Actix Handler for Lock message.
impl Handler<Lock> for RedisActor {
    type Result = ResponseFuture<Result<LockStatus, Error>>;

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
        })
    }
}
