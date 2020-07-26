use actix::prelude::*;
use actix_derive::{Message, MessageResponse};
use actix_cache::{actor::Cache, error::CacheError, cache::Cacheable};
use serde::{Serialize, Deserialize};
use env_logger;

struct UpstreamActor;

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

#[derive(MessageResponse, Deserialize, Serialize, Debug)]
struct Pong(i32);

#[derive(Message)]
#[rtype(result = "Result<Pong, ()>")]
struct Ping(i32);

impl Cacheable for Ping {
    fn cache_key(&self) -> String {
        format!("Ping::{}", self.0)
    }
}

impl Handler<Ping> for UpstreamActor {
    type Result = ResponseFuture<<Ping as Message>::Result>;

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            actix_rt::time::delay_for(core::time::Duration::from_secs(3)).await;
            Ok(Pong(msg.0))
        })
    }
}

use actix_cache_backend::{BackendError, Backend, Set, Get, Delete, Lock, DeleteStatus, LockStatus};

struct DummyBackend;

impl Actor for DummyBackend {
    type Context = Context<Self>;
}

impl Backend for DummyBackend {
    type Actor = Self;
    type Context = Context<Self>;
}


impl Handler<Get> for DummyBackend {
    type Result = ResponseFuture<Result<Option<String>, BackendError>>;

    fn handle(&mut self, _msg: Get, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy backend GET");
        let fut = async move {
            Ok(None)
        };
        Box::pin(fut)
    }
}

impl Handler<Set> for DummyBackend {
    type Result = Result<String, BackendError>;

    fn handle(&mut self, _msg: Set, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy backend SET");
        Ok("42".to_owned())
    }
}

impl Handler<Delete> for DummyBackend {
    type Result = ResponseFuture<Result<DeleteStatus, BackendError>>;

    fn handle(&mut self, _msg: Delete, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy backend Delete");
        let fut = async move {
            Ok(DeleteStatus::Missing)
        };
        Box::pin(fut)
    }
}

impl Handler<Lock> for DummyBackend {
    type Result = ResponseFuture<Result<LockStatus, BackendError>>;

    fn handle(&mut self, _msg: Lock, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy backend Lock");
        let fut = async move {
            Ok(LockStatus::Acquired)
        };
        Box::pin(fut)
    }
}

struct DummySyncBackend;

impl Actor for DummySyncBackend {
    type Context = SyncContext<Self>;
}

impl Backend for DummySyncBackend {
    type Actor = Self;
    type Context = SyncContext<Self>;
}


impl Handler<Get> for DummySyncBackend {
    type Result = Result<Option<String>, BackendError>;

    fn handle(&mut self, _msg: Get, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy sync backend GET");
        Ok(None)
    }
}

impl Handler<Set> for DummySyncBackend {
    type Result = Result<String, BackendError>;

    fn handle(&mut self, _msg: Set, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy sync backend SET");
        Ok("42".to_owned())
    }
}

impl Handler<Delete> for DummySyncBackend {
    type Result = Result<DeleteStatus, BackendError>;

    fn handle(&mut self, _msg: Delete, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy sync backend Delete");
        Ok(DeleteStatus::Missing)
    }
}

impl Handler<Lock> for DummySyncBackend {
    type Result = Result<LockStatus, BackendError>;

    fn handle(&mut self, _msg: Lock, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy sync backend Lock");
        Ok(LockStatus::Acquired)
    }
}

#[actix_rt::main]
async fn main() -> Result<(), CacheError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    use actix_cache_redis::actor::RedisActor;
    let backend = RedisActor::new()
        .await
        .map_err(|err| CacheError::BackendError(err.into()))?
        .start();

    let dummy_backend = DummyBackend.start();
    let dummy_sync_backend = {
        SyncArbiter::start(3, move || DummySyncBackend)
    };

    // let cache = Cache::new(dummy_backend)
    let cache = Cache::builder()
        .build(dummy_sync_backend)
        .start();
    let upstream = UpstreamActor.start(); 

    let msg = Ping(42);
    let res = cache.send(msg.into_cache(upstream))
        .await??;
    dbg!(res.unwrap());

    Ok(())
}
