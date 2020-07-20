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
#[rtype(result = "Pong")]
struct Ping(i32);

impl Cacheable for Ping {
    fn cache_key(&self) -> String {
        format!("Ping::{}", self.0)
    }
}

impl Handler<Ping> for UpstreamActor {
    type Result = <Ping as actix::Message>::Result;
    fn handle(&mut self, msg: Ping, _: &mut Self::Context) -> Pong {
        Pong(msg.0)
    }
}

#[actix_rt::main]
async fn main() -> Result<(), CacheError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let cache = Cache::new().await?.start();
    let upstream = UpstreamActor.start(); 

    let msg = Ping(42);
    let res = cache.send(msg.into_cache(upstream))
        .await?;
    dbg!(res);

    Ok(())
}
