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

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "Result<Pong, ()>")]
struct Ping {
    pub id: i32
}

impl Handler<Ping> for UpstreamActor {
    type Result = ResponseFuture<<Ping as Message>::Result>;

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            actix_rt::time::delay_for(core::time::Duration::from_secs(3)).await;
            Ok(Pong(msg.id))
        })
    }
}

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

#[actix_rt::main]
async fn main() -> Result<(), CacheError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let dummy_backend = DummyBackend.start();

    let cache = Cache::builder()
        .build(dummy_backend)
        .start();
    let upstream = UpstreamActor.start(); 

    let msg = Ping { id: 42 };
    let res = cache.send(msg.into_cache(upstream))
        .await??;
    dbg!(res.unwrap());

    Ok(())
}
