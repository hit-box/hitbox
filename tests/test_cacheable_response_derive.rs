use actix_cache::response::{CacheableResponse, CachePolicy};
use serde::Serialize;

#[derive(CacheableResponse, Serialize, Clone, Debug, Eq, PartialEq)]
struct Message {
    id: i32,
    alias: String,
}

#[test]
fn test_custom_message_into_policy() {
    let message = Message { id: 0, alias: String::from("alias") };
    let policy = message.clone().into_policy();
    match policy {
        CachePolicy::Cacheable(value) => assert_eq!(value, message),
        CachePolicy::NonCacheable(_) => assert!(false),
    };
}
