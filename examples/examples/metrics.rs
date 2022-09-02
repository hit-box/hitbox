use actix::prelude::*;
use actix_derive::MessageResponse;
use hitbox::{hitbox_serializer, CachePolicy, Cacheable, CacheableResponse};
use hitbox_actix::{Cache, CacheError, IntoCache};
use metrics_exporter_prometheus::PrometheusBuilder;

#[derive(Default)]
struct UpstreamActor;

impl Actor for UpstreamActor {
    type Context = Context<Self>;
}

#[derive(Message, Cacheable, serde::Serialize)]
#[rtype(result = "Pong")]
struct Ping {
    number: u8,
}

impl Ping {
    fn new(number: u8) -> Self {
        Self { number }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, MessageResponse, CacheableResponse)]
struct Pong {
    number: u8,
}

impl Handler<Ping> for UpstreamActor {
    type Result = MessageResult<Ping>;

    fn handle(&mut self, msg: Ping, _: &mut Self::Context) -> Self::Result {
        MessageResult(Pong { number: msg.number })
    }
}

#[actix::main]
async fn main() {
    let upstream = UpstreamActor::default().start();
    let cache = Cache::new().await.unwrap().start();
    let recorder = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install recorder");

    let _pong_0 = cache
        .send(Ping::new(0).into_cache(&upstream))
        .await
        .expect("actix mailbox timeout/closed")
        .expect("cache actor error");

    let _again_pong_0 = cache
        .send(Ping::new(0).into_cache(&upstream))
        .await
        .expect("actix mailbox timeout/closed")
        .expect("cache actor error");

    let _pong_1 = cache
        .send(Ping::new(1).into_cache(&upstream))
        .await
        .expect("actix mailbox timeout/closed")
        .expect("cache actor error");

    println!("{}", recorder.render());
}
