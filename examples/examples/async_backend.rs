/*use actix::prelude::*;
use hitbox::dev::{Backend, BackendError, Delete, DeleteStatus, Get, Lock, LockStatus, Set};
use hitbox_actix::prelude::*;
use serde::{Deserialize, Serialize};

struct UpstreamActor;

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

#[derive(MessageResponse, Deserialize, Serialize, Debug)]
struct Pong(i32);

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "Result<Pong, ()>")]
struct Ping {
    pub id: i32,
}

impl Handler<Ping> for UpstreamActor {
    type Result = ResponseFuture<<Ping as Message>::Result>;

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            actix_rt::time::sleep(core::time::Duration::from_secs(3)).await;
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
    type Result = ResponseFuture<Result<Option<Vec<u8>>, BackendError>>;

    fn handle(&mut self, _msg: Get, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy backend GET");
        let fut = async move { Ok(None) };
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
        let fut = async move { Ok(DeleteStatus::Missing) };
        Box::pin(fut)
    }
}

impl Handler<Lock> for DummyBackend {
    type Result = ResponseFuture<Result<LockStatus, BackendError>>;

    fn handle(&mut self, _msg: Lock, _: &mut Self::Context) -> Self::Result {
        log::warn!("Dummy backend Lock");
        let fut = async move { Ok(LockStatus::Acquired) };
        Box::pin(fut)
    }
}

#[actix::main]
async fn main() -> Result<(), CacheError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let dummy_backend = DummyBackend.start();

    let cache = CacheActor::builder().finish(dummy_backend).start();
    let upstream = UpstreamActor.start();

    let msg = Ping { id: 42 };
    let _ = cache.send(msg.into_cache(&upstream)).await??;

    Ok(())
}*/

fn main() {

}
