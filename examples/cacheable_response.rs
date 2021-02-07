use actix::prelude::*;
use actix_cache::{Cache, CacheError, Cacheable};
use actix_derive::{Message, MessageResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct UpstreamActor;

#[derive(Debug)]
enum Error {
    Test,
}

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

#[derive(MessageResponse, Deserialize, Serialize, Debug, Clone)]
struct Pong(i32);

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "Result<Pong, Error>")]
struct Ping {
    id: i32,
}

// impl<I, E> UpstreamResponse<'a> for Ping 
// where
    // Self: Message<Result = Result<I, E>>,
// {
    // type UpstreamValue = &'a I;
    // type UpstreamError = &'a E;
    // type CacheableResult = Result<UpstreamValue, UpstreamError>;
    
    // fn into_upstream_result(result: &'a Self::Result) -> Self::CacheableResult {
        // result.as_ref()
    // }
// }

impl Handler<Ping> for UpstreamActor {
    type Result = ResponseFuture<<Ping as Message>::Result>;

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            actix_rt::time::delay_for(core::time::Duration::from_secs(3)).await;
            Ok(Pong(msg.id))
            // Err(Error::Test)
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
    dbg!(res);
    Ok(())
}
