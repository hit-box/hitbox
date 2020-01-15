use actix::prelude::*;
use log::{info, debug};
use crate::error::Error;
use redis::{Client, aio::MultiplexedConnection};

/// Actix Redis cache backend actor.
pub struct Redis {
    #[allow(dead_code)]
    client: Client,
    con: MultiplexedConnection,
}

impl Redis {
    /// Create new Redis cache backend actor instance.
    pub async fn new() -> Self {
        let client = Client::open("redis://127.0.0.1/").unwrap();
        let con = client.get_multiplexed_tokio_connection().await.unwrap();
        Redis { client, con }
    }
}

/// Implementation actix Actor trait for Redis cache backend.
impl Actor for Redis {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
    }
}

/// Actix message implements request Redis value by key.
#[derive(Message, Debug)]
#[rtype(result="Result<Option<String>, Error>")]
pub struct Get {
    pub key: String,
}

/// Implementation of Actix Handler for Get message.
impl Handler<Get> for Redis {
    type Result = ResponseFuture<Result<Option<String>, Error>>;

    fn handle(&mut self, msg: Get, _: &mut Self::Context) -> Self::Result {
        let mut con = self.con.clone();
        let fut = async move {
            redis::cmd("GET")
                .arg(msg.key)
                .query_async(&mut con)
                .await
                .map_err(Error::from)
        };
        Box::pin(fut)
    }
}

/// Actix message implements writing Redis value by key.
#[derive(Message, Debug, Clone)]
#[rtype(result="Result<String, Error>")]
pub struct Set {
    pub key: String,
    pub value: String,
    pub ttl: Option<u32>,
}

/// Implementation of Actix Handler for Set message.
impl Handler<Set> for Redis {
    type Result = ResponseFuture<Result<String, Error>>;

    fn handle(&mut self, msg: Set, _: &mut Self::Context) -> Self::Result {
        let mut con = self.con.clone();
        Box::pin(async move {
            let mut request = redis::cmd("SET");
            request
                .arg(msg.key)
                .arg(msg.value);
            if let Some(ttl) = msg.ttl {
                request
                    .arg("EX")
                    .arg(ttl);
            };
            request
                .query_async(&mut con)
                .await
                .map_err(Error::from)
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
#[rtype(result="Result<DeleteStatus, Error>")]
pub struct Delete {
    pub key: String,
}

/// Implementation of Actix Handler for Delete message.
impl Handler<Delete> for Redis {
    type Result = ResponseFuture<Result<DeleteStatus, Error>>;

    fn handle(&mut self, msg: Delete, _: &mut Self::Context) -> Self::Result {
        let mut con = self.con.clone();
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
#[rtype(result="Result<LockStatus, Error>")]
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
impl Handler<Lock> for Redis {
    type Result = ResponseFuture<Result<LockStatus, Error>>;

    fn handle(&mut self, msg: Lock, _: &mut Self::Context) -> Self::Result {
        debug!("Redis Lock: {}", msg.key);
        let mut con = self.con.clone();
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
