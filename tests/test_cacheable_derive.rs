use actix_cache::cache::*;
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

#[derive(Serialize)]
enum TupleMessageType {
    External(i32),
}

#[derive(Cacheable, Serialize)]
struct TupleEnumMessage {
    message_type: TupleMessageType
}

#[test]
fn test_message_with_enum_tuple() {
    let message = TupleEnumMessage { message_type: TupleMessageType::External(1) };
    assert_eq!(message.cache_key(), "message_type[External]=1".to_string());
}

#[derive(Cacheable, Serialize)]
#[cache_ttl(42)]
#[cache_stale_ttl(10)]
#[cache_version(1)]
struct TTLMessage {
    message_type: i32
}

#[test]
fn test_message_ttl() {
    let message = TTLMessage { message_type: 1 };
    assert_eq!(message.cache_ttl(), 42);
    assert_eq!(message.cache_stale_ttl(), 32);
    assert_eq!(message.cache_version(), 1);
}
