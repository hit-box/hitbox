use actix::prelude::*;
use actix_cache::{Cache, dev::backend::MockBackend, Cacheable, CacheError};
use serde::{Serialize, Deserialize};

struct UpstreamActor;

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

#[derive(MessageResponse, Deserialize, Serialize, Debug)]
struct Pong { 
    id:i32
}

#[derive(Message, Serialize)]
#[rtype(result = "Pong")]
struct Ping {
    pub id: i32
}

impl Cacheable for Ping {
    fn cache_key(&self) -> Result<String, CacheError> {
        Ok(format!("Ping::{}", self.id))
    }
}

impl Handler<Ping> for UpstreamActor {
    type Result = <Ping as Message>::Result;

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        Pong { id: msg.id }
    }
}

#[actix_rt::test]
async fn test_mock_backend() {
    let backend = MockBackend::new();
    let addr = backend.start();
    let cache = Cache::builder()
        .build(addr.clone())
        .start();
    let upstream = UpstreamActor.start(); 
    let msg = Ping { id: 42 };
    cache.send(msg.into_cache(upstream))
        .await.unwrap().unwrap();
}
