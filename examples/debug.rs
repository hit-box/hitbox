use actix::prelude::*;
use actix_derive::{Message, MessageResponse};
use actix_cache::{actor::Cache, error::CacheError, cache::Cacheable};
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
    id: i32
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
    let res = cache.send(msg.into_cache(upstream))
        .await??;
    dbg!(res.unwrap());

    Ok(())
}
