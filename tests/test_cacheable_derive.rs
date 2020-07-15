use actix_cache::cache::Cacheable;
use serde::Serialize;

#[derive(Cacheable, Serialize)]
struct Message {
    id: i32,
    alias: String,
}

#[test]
fn test_all_keys() {
    let message = Message { id: 0, alias: "alias".to_string() };
    assert_eq!(message.cache_key(), "id=0&alias=alias".to_string());
}


#[derive(Cacheable, Serialize)]
#[allow(dead_code)]
struct PartialSerializeMessage {
    id: i32,
    #[serde(skip_serializing)]
    alias: String,
}

#[test]
fn test_partial() {
    let message = PartialSerializeMessage { id: 0, alias: "alias".to_string() };
    assert_eq!(message.cache_key(), "id=0".to_string());
}
