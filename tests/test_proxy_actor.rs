use actix::prelude::*;
use actix_cache::cache::Cacheable;
use actix_cache::actor::Cache;

pub struct Upstream;

impl Actor for Upstream {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        log::info!("Cache actor started");
    }
}

#[derive(Message)]
#[rtype(result = "Result<i32, ()>")]
pub struct Ping;

impl Cacheable for Ping {
    fn cache_key(&self) -> String {
        "Ping::".to_owned()
    }
}

impl Handler<Ping> for Upstream {
    type Result = ResponseFuture<Result<i32, ()>>;

    fn handle(&mut self, _msg: Ping, _: &mut Self::Context) -> Self::Result {
        Box::pin(async {
            Ok(42)
        })
    }
}

#[derive(Message)]
#[rtype(result = "i32")]
pub struct Pong;

impl Cacheable for Pong {
    fn cache_key(&self) -> String {
        "Pong::".to_owned()
    }
}

impl Handler<Pong> for Upstream {
    type Result = i32;

    fn handle(&mut self, _msg: Pong, _: &mut Self::Context) -> Self::Result {
        42
    }
}

struct SyncUpstream;

impl Actor for SyncUpstream {
    type Context = SyncContext<Self>;
}

impl Handler<Ping> for SyncUpstream {
    type Result = Result<i32, ()>;

    fn handle(&mut self, _msg: Ping, _: &mut Self::Context) -> Self::Result {
        Ok(42)
    }
}

#[actix_rt::test]
async fn test_async_proxy() {
    let cache = Cache::new().start();
    let upstream = Upstream {}.start();
    let res = cache.send(Ping {}.into_cache(upstream.clone())).await.unwrap();
    assert_eq!(res, Ok(42));
    // let res = cache.send(QueryCache { message: Pong {}, upstream: test}).await.unwrap();
    // assert_eq!(res, 42);
}

#[actix_rt::test]
async fn test_sync_proxy() {
    let upstream = SyncArbiter::start(10, move || SyncUpstream {});
    let cache = Cache::new().start();
    // let res = cache.send(QueryCache { message: Pong {}, upstream: test}).await.unwrap();
    // assert_eq!(res, 42);
    let res = cache.send(Ping {}.into_cache(upstream)).await.unwrap();
    assert_eq!(res, Ok(42));
}
