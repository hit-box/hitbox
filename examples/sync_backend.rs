use actix::prelude::*;
use actix_cache::{Cache, CacheError, Cacheable};
use actix_cache::dev::{Backend, BackendError, Get, Set, Delete, DeleteStatus, Lock, LockStatus};
use serde::{Serialize, Deserialize};

struct UpstreamActor;

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

#[derive(MessageResponse, Deserialize, Serialize, Debug)]
struct Pong(i32);

impl Cacheable for Ping {
    fn cache_key(&self) -> String {
        format!("Ping::{}", self.0)
    }
}

#[derive(Message)]
#[rtype(result = "Result<Pong, ()>")]
struct Ping(i32);

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

    // let dummy_backend = DummySyncBackend.start();
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
