use actix::prelude::*;
use actix_derive::{Message, MessageResponse};
use hitbox::{Cache, CacheError, Cacheable};
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

fn test<T>(value: T) {}

#[derive(MessageResponse, Deserialize, Serialize, Debug)]
struct Pong(i32);

#[derive(Debug)]
enum Error {}

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "Result<Pong, Error>")]
struct Ping {
    id: i32,
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

#[actix_rt::main]
async fn main() -> Result<(), CacheError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let cache = Cache::new().await?.start();
    let upstream = UpstreamActor.start();

    let msg = Ping { id: 42 };
    let res = cache.send(msg.into_cache(&upstream)).await??;
    Ok(())
}
