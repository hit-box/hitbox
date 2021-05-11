use hitbox::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    id: i32,
    alias: String,
}

impl Cacheable for Message {
    fn cache_key(&self) -> Result<String, CacheError> {
        Ok("overloaded cache key".to_owned())
    }

    fn cache_key_prefix(&self) -> String {
        "Message".to_owned()
    }
}

#[actix_rt::test]
async fn test_cache_key() {
    let message = Message {
        id: 42,
        alias: "test".to_owned(),
    };
    assert_eq!(
        message.cache_key().unwrap().as_str(),
        "overloaded cache key"
    );
    assert_eq!(
        message.cache_key_prefix().as_str(),
        "Message"
    );
    let message = Message {
        id: 28,
        alias: "cow level".to_owned(),
    };
    assert_eq!(
        message.cache_key().unwrap().as_str(),
        "overloaded cache key"
    );
    assert_eq!(
        message.cache_key_prefix().as_str(),
        "Message"
    );
}
