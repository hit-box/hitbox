use actix::prelude::*;
use actix_derive::{Message, MessageResponse};
use hitbox_actix::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct UpstreamActor;

#[derive(Debug)]
struct Error;

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

#[derive(MessageResponse, Deserialize, Serialize, Debug)]
struct Pong(i32);

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "Result<Pong, Error>")]
struct Ping {
    id: i32,
}

impl Handler<Ping> for UpstreamActor {
    type Result = ResponseFuture<<Ping as Message>::Result>;

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        println!("Handler::Ping");
        Box::pin(async move {
            actix_rt::time::sleep(core::time::Duration::from_secs(3)).await;
            Ok(Pong(msg.id))
        })
    }
}

use tracing_subscriber::EnvFilter;

#[actix_rt::main]
async fn main() -> Result<(), CacheError> {
    let filter = EnvFilter::new("hitbox=trace");
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_env_filter(filter)
        .init();

    let backend = RedisBackend::new().await.unwrap().start();

    let cache = Cache::builder()
        .with_stale()
        .without_lock()
        .finish(backend)
        .start();
    let upstream = UpstreamActor.start();

    let msg = Ping { id: 42 };
    let res = cache.send(msg.into_cache(&upstream)).await??;
    println!("{:#?}", res);
    Ok(())
}
