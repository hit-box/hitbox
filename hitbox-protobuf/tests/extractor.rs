mod common;

use std::collections::HashMap;

use bytes::Bytes;
use http_body_util::Empty;
use prost_reflect::{MapKey, Value};

use common::{TestRequest, request_from_bytes};
use hitbox::Extractor;
use hitbox_http::extractors::{self, MethodConfig, MethodExtractor};
use hitbox_protobuf::ProtoFieldsExtract;

const METHOD: &str = "POST";
const URI: &str = "/test.UserService/ListUsers";
const MESSAGE: &str = "test.ListUsersRequest";

mod happy_path {
    use super::*;

    #[tokio::test]
    async fn extracts_field_values_as_key_parts() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(3))
            .field("page_size", Value::I32(25))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let extractor =
            extractors::extractor().proto_fields(desc, vec!["role".into(), "page".into()]);

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].key(), "role");
        assert_eq!(parts[0].value(), Some("admin"));
        assert_eq!(parts[1].key(), "page");
        assert_eq!(parts[1].value(), Some("3"));
    }

    #[tokio::test]
    async fn extracts_single_field() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("viewer".into()))
            .build_request(METHOD, URI);

        let extractor =
            extractors::extractor::<Empty<Bytes>>().proto_fields(desc, vec!["role".into()]);

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "role");
        assert_eq!(parts[0].value(), Some("viewer"));
    }

    #[tokio::test]
    async fn chains_with_method_extractor() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let extractor = extractors::extractor::<Empty<Bytes>>()
            .method(MethodConfig::new())
            .proto_fields(desc, vec!["role".into()]);

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].key(), "method");
        assert_eq!(parts[0].value(), Some("POST"));
        assert_eq!(parts[1].key(), "role");
        assert_eq!(parts[1].value(), Some("admin"));
    }
}

mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn missing_field_produces_none_value() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let extractor = extractors::extractor::<Empty<Bytes>>()
            .proto_fields(desc, vec!["nonexistent_field".into()]);

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "nonexistent_field");
        assert_eq!(parts[0].value(), None);
    }

    #[tokio::test]
    async fn invalid_protobuf_yields_no_extra_parts() {
        let desc = TestRequest::new(MESSAGE).message_descriptor();
        let request = request_from_bytes(
            METHOD,
            URI,
            Bytes::from_static(b"not valid protobuf bytes!!!"),
        );

        let extractor =
            extractors::extractor::<Empty<Bytes>>().proto_fields(desc, vec!["role".into()]);

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert!(parts.is_empty());
    }

    #[tokio::test]
    async fn body_is_preserved_after_extraction() {
        let builder = TestRequest::new(MESSAGE)
            .field("page", Value::I32(7))
            .field("page_size", Value::I32(50))
            .field("role", Value::String("editor".into()));
        let original_bytes = builder.build_bytes();
        let (desc, request) = builder.build_request(METHOD, URI);

        let extractor =
            extractors::extractor::<Empty<Bytes>>().proto_fields(desc, vec!["role".into()]);

        let key_parts = extractor.get(request).await;
        let (request, _cache_key) = key_parts.into_cache_key();

        let (_, body) = request.into_parts();
        let collected = body.collect().await.expect("body should be collectible");
        assert_eq!(collected.data, original_bytes);
    }
}

mod complex_types {
    use super::*;

    #[tokio::test]
    async fn extracts_map_value_by_key() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("role", Value::String("admin".into()))
            .field(
                "metadata",
                Value::Map(HashMap::from([
                    (
                        MapKey::String("env".into()),
                        Value::String("production".into()),
                    ),
                    (
                        MapKey::String("region".into()),
                        Value::String("us-east".into()),
                    ),
                ])),
            )
            .build_request(METHOD, URI);

        let extractor =
            extractors::extractor::<Empty<Bytes>>().proto_fields(desc, vec!["metadata.env".into()]);

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "metadata.env");
        assert_eq!(parts[0].value(), Some("production"));
    }

    #[tokio::test]
    async fn extracts_repeated_field_sorted() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("role", Value::String("admin".into()))
            .field(
                "tags",
                Value::List(vec![
                    Value::String("urgent".into()),
                    Value::String("important".into()),
                    Value::String("bug".into()),
                ]),
            )
            .build_request(METHOD, URI);

        let extractor =
            extractors::extractor::<Empty<Bytes>>().proto_fields(desc, vec!["tags".into()]);

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "tags");
        assert_eq!(parts[0].value(), Some("bug,important,urgent"));
    }

    #[tokio::test]
    async fn extracts_enum_by_name() {
        let pool = common::test_pool();
        let user_desc = pool.get_message("test.User").unwrap();
        let mut user = prost_reflect::DynamicMessage::new(user_desc.clone());

        use prost_reflect::ReflectMessage;
        let name_fd = user.descriptor().get_field_by_name("name").unwrap();
        let role_fd = user.descriptor().get_field_by_name("role").unwrap();
        user.set_field(&name_fd, Value::String("alice".into()));
        user.set_field(&role_fd, Value::EnumNumber(1)); // ADMIN = 1

        use prost::Message;
        let body_bytes = Bytes::from(user.encode_to_vec());
        let request = request_from_bytes(METHOD, URI, body_bytes);

        let extractor = extractors::extractor::<Empty<Bytes>>()
            .proto_fields(user_desc, vec!["role".into(), "name".into()]);

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].key(), "role");
        assert_eq!(parts[0].value(), Some("ADMIN"));
        assert_eq!(parts[1].key(), "name");
        assert_eq!(parts[1].value(), Some("alice"));
    }

    #[tokio::test]
    async fn extracts_nested_message_field() {
        let pool = common::test_pool();
        let user_desc = pool.get_message("test.User").unwrap();
        let addr_desc = pool.get_message("test.Address").unwrap();

        let mut addr = prost_reflect::DynamicMessage::new(addr_desc);
        use prost_reflect::ReflectMessage;
        let city_fd = addr.descriptor().get_field_by_name("city").unwrap();
        let country_fd = addr.descriptor().get_field_by_name("country").unwrap();
        addr.set_field(&city_fd, Value::String("NYC".into()));
        addr.set_field(&country_fd, Value::String("US".into()));

        let mut user = prost_reflect::DynamicMessage::new(user_desc.clone());
        let name_fd = user.descriptor().get_field_by_name("name").unwrap();
        let addr_fd = user.descriptor().get_field_by_name("address").unwrap();
        user.set_field(&name_fd, Value::String("alice".into()));
        user.set_field(&addr_fd, Value::Message(addr));

        use prost::Message;
        let body_bytes = Bytes::from(user.encode_to_vec());
        let request = request_from_bytes(METHOD, URI, body_bytes);

        let extractor = extractors::extractor::<Empty<Bytes>>().proto_fields(
            user_desc,
            vec!["address.city".into(), "address.country".into()],
        );

        let key_parts = extractor.get(request).await;
        let (_request, cache_key) = key_parts.into_cache_key();
        let parts: Vec<_> = cache_key.parts().collect();

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].key(), "address.city");
        assert_eq!(parts[0].value(), Some("NYC"));
        assert_eq!(parts[1].key(), "address.country");
        assert_eq!(parts[1].value(), Some("US"));
    }
}
