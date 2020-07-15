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

#[derive(Cacheable, Serialize)]
struct VecMessage {
    id: Vec<i32>,
}

#[test]
fn test_message_with_vector() {
    let message = VecMessage { id: vec![1, 2, 3] };
    assert_eq!(message.cache_key(), "id[0]=1&id[1]=2&id[2]=3".to_string());
}

#[derive(Serialize)]
enum MessageType {
    External,
}

#[derive(Cacheable, Serialize)]
struct EnumMessage {
    message_type: MessageType
}

#[test]
fn test_message_with_enum() {
    let message = EnumMessage { message_type: MessageType::External };
    assert_eq!(message.cache_key(), "message_type=External".to_string());
}
