use hitbox::{CachePolicy, CacheableResponse};
use serde::Serialize;

#[derive(CacheableResponse, Serialize, Clone, Debug, Eq, PartialEq)]
struct Message {
    id: i32,
    alias: String,
}

#[test]
fn test_custom_message_into_policy() {
    let message = Message {
        id: 0,
        alias: String::from("alias"),
    };
    let policy = message.clone().into_cache_policy();
    match policy {
        CachePolicy::Cacheable(value) => assert_eq!(value, message),
        CachePolicy::NonCacheable(_) => panic!(),
    };
}

#[derive(CacheableResponse, Serialize, Clone, Debug, Eq, PartialEq)]
enum EnumMessage {
    Variant(i32),
}

#[test]
fn test_custom_enum_message_into_policy() {
    let message = EnumMessage::Variant(1);
    let policy = message.clone().into_cache_policy();
    match policy {
        CachePolicy::Cacheable(value) => assert_eq!(value, message),
        CachePolicy::NonCacheable(_) => panic!(),
    };
}
