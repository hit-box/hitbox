use actix::prelude::*;
use log::info;
use crate::error::Error;
use redis::{Client, aio::MultiplexedConnection};

pub struct Redis {
    #[allow(dead_code)]
    client: Client,
    con: MultiplexedConnection,
}

impl Redis {
    pub async fn new() -> Self {
        let client = Client::open("redis://127.0.0.1/").unwrap();
        let con = client.get_multiplexed_tokio_connection().await.unwrap();
        Redis { client, con }
    }
}

impl Actor for Redis {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
    }
}

#[derive(Message, Debug)]
#[rtype(result="Result<Option<String>, Error>")]
pub struct Get {
    pub key: String,
}

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

#[derive(Message, Debug, Clone)]
#[rtype(result="Result<String, Error>")]
pub struct Set {
    pub key: String,
    pub value: String,
    pub ttl: Option<u32>,
}

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

#[derive(Message, Debug)]
#[rtype(result="Result<DeleteStatus, Error>")]
pub struct Delete {
    pub key: String,
}

impl Handler<Delete> for Redis {
    type Result = ResponseFuture<Result<DeleteStatus, Error>>;

    fn handle(&mut self, msg: Delete, _: &mut Self::Context) -> Self::Result {
        let mut con = self.con.clone();
        Box::pin(async move {
            let mut request = redis::cmd("DEL");
            request
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
