use actix::prelude::*;
use actix_derive::{Message, MessageResponse};
use hitbox_actix::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct UpstreamActor;

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

enum CacheableResult<T, U> {
    Cacheable(T),
    NoneCacheable(U),
}

trait CacheableResponse<T, E> {
    fn cache(&self) -> CacheableResult<&T, &E>;
}

impl<E> CacheableResponse<i32, E> for i32 {
    fn cache(&self) -> CacheableResult<&i32, &E> {
        CacheableResult::Cacheable(self)
    }
}

impl<T, E> CacheableResponse<T, E> for Result<T, E> {
    fn cache(&self) -> CacheableResult<&T, &E> {
        match self {
            Ok(value) => CacheableResult::Cacheable(value),
            Err(value) => CacheableResult::NoneCacheable(value),
        }
    }
}

#[derive(MessageResponse, Deserialize, Serialize, Debug)]
struct Pong(i32);

#[derive(Debug)]
struct PongError {}

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "Result<Pong, PongError>")]
struct Ping {
    id: i32,
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

#[derive(Debug)]
enum Error {
    Actix(actix::MailboxError),
    Cache(hitbox::CacheError),
    Msg(PongError),
}

impl From<actix::MailboxError> for Error {
    fn from(err: actix::MailboxError) -> Error {
        Error::Actix(err)
    }
}

impl From<hitbox::CacheError> for Error {
    fn from(err: hitbox::CacheError) -> Error {
        Error::Cache(err)
    }
}

impl From<PongError> for Error {
    fn from(err: PongError) -> Error {
        Error::Msg(err)
    }
}

#[actix::main]
async fn main() -> Result<(), Error> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let cache = Cache::new().await?.start();
    let upstream = UpstreamActor.start();

    let msg = Ping { id: 42 };
    let _ = cache.send(msg.into_cache(&upstream)).await???;
    Ok(())
}
