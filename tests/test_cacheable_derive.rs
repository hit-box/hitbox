#[cfg(feature = "derive")]
mod tests {
    use actix_cache::cache::Cacheable;
    use serde::Serialize;
    use actix_cache::CacheError;

    #[derive(Cacheable, Serialize)]
    struct Message {
        id: i32,
        alias: String,
    }

    #[test]
    fn test_all_keys() {
        let message = Message { id: 0, alias: "alias".to_string() };
        assert_eq!(message.cache_key().unwrap(), "id=0&alias=alias".to_string());
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
        assert_eq!(message.cache_key().unwrap(), "id=0".to_string());
    }

    #[derive(Cacheable, Serialize)]
    struct VecMessage {
        id: Vec<i32>,
    }

    #[test]
    fn test_message_with_vector() {
        let message = VecMessage { id: vec![1, 2, 3] };
        assert_eq!(message.cache_key().unwrap(), "id[0]=1&id[1]=2&id[2]=3".to_string());
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
        assert_eq!(message.cache_key().unwrap(), "message_type=External".to_string());
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
        assert_eq!(message.cache_key().unwrap(), "message_type[External]=1".to_string());
    }

    // Should we support tuple struct?
    #[derive(Cacheable, Serialize)]
    struct TupleMessage(i32);

    #[test]
    fn test_tuple_returns_error() {
        let message = TupleMessage(1);
        assert!(message.cache_key().is_err());
    }

    #[derive(Cacheable, Serialize)]
    #[cache_ttl(42)]
    #[cache_stale_ttl(30)]
    #[cache_version(1)]
    struct MacroHelpersMessage {
        message_type: i32
    }

    #[test]
    fn test_macro_helpers_work() {
        let message = MacroHelpersMessage { message_type: 1 };
        assert_eq!(message.cache_ttl(), 42);
        assert_eq!(message.cache_stale_ttl(), 30);
        assert_eq!(message.cache_version(), 1);
    }

    #[derive(Cacheable, Serialize)]
    struct DefaultMessage {
        message_type: i32
    }

    #[test]
    fn test_default_ttl_stale_ttl_version_work() {
        let message = DefaultMessage { message_type: 1 };
        assert_eq!(message.cache_ttl(), 60);
        assert_eq!(message.cache_stale_ttl(), 55);
        assert_eq!(message.cache_version(), 0);
    }
}
