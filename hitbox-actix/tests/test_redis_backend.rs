use actix::prelude::*;
use hitbox::{
    dev::CacheBackend, CacheError, CachePolicy, Cacheable, CacheableResponse, CachedValue,
};
use hitbox_actix::prelude::*;
use serde::{Deserialize, Serialize};

struct UpstreamActor;

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

#[derive(MessageResponse, CacheableResponse, Deserialize, Serialize, Debug, PartialEq)]
struct Pong {
    id: i32,
}

#[derive(Message, Serialize)]
#[rtype(result = "Pong")]
struct Ping {
    pub id: i32,
}

impl Cacheable for Ping {
    fn cache_key(&self) -> Result<String, CacheError> {
        Ok(format!("{}::{}", self.cache_key_prefix(), self.id))
    }
    fn cache_key_prefix(&self) -> String {
        "Ping".to_owned()
    }
}

impl Handler<Ping> for UpstreamActor {
    type Result = <Ping as Message>::Result;

    fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        Pong { id: msg.id }
    }
}

#[actix::test]
async fn test_mock_backend() {
    let backend = RedisBackend::new().await.unwrap();
    let cache = CacheActor::builder().finish(backend).start();
    let upstream = UpstreamActor.start();
    let msg = Ping { id: 42 };
    cache
        .send(msg.into_cache(&upstream))
        .await
        .unwrap()
        .unwrap();

    let backend = RedisBackend::new().await.unwrap();
    let res: Pong = backend
        .get("UpstreamActor::Ping::42".to_owned())
        .await
        .unwrap()
        .unwrap()
        .into_inner();
    assert_eq!(res, Pong { id: 42 });

}
