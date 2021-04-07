use actix::prelude::*;
use actix_cache::cache::Cacheable;
use serde::Serialize;

#[derive(Cacheable, Serialize, Message)]
#[rtype(result = "String")]
struct Message {
    id: i32,
    alias: String,
}

struct Upstream;

impl Actor for Upstream {
    type Context = Context<Self>;
}

impl Handler<Message> for Upstream {
    type Result = ResponseFuture<String>;

    fn handle(&mut self, _msg: Message, _: &mut Self::Context) -> Self::Result {
        Box::pin(async {
            "Upstream".to_owned()
        })
    }
}

struct NextUpstream;

impl Actor for NextUpstream {
    type Context = Context<Self>;
}

impl Handler<Message> for NextUpstream {
    type Result = ResponseFuture<String>;

    fn handle(&mut self, _msg: Message, _: &mut Self::Context) -> Self::Result {
        Box::pin(async {
            "NextUpstream".to_owned()
        })
    }
}

#[actix_rt::test]
async fn test_final_cache_key() {
    let upstream = Upstream.start();
    let next_upstream = NextUpstream.start();
    let message = Message { id: 42, alias: "test".to_owned() }.into_cache(&upstream);
    assert_eq!(message.cache_key().unwrap().as_str(), "Upstream::Message::v0::id=42&alias=test");
    let message = Message { id: 28, alias: "cow level".to_owned() }.into_cache(&next_upstream);
    assert_eq!(message.cache_key().unwrap().as_str(), "NextUpstream::Message::v0::id=28&alias=cow+level");
}
