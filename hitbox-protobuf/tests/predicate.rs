mod common;

use std::collections::HashMap;

use bytes::Bytes;
use http_body_util::Empty;
use prost_reflect::{MapKey, ReflectMessage, Value};
use regex::Regex;

use common::{TestRequest, request_from_bytes};
use hitbox::predicate::{Predicate, PredicateExt, PredicateResult};
use hitbox_http::predicates::request;
use hitbox_protobuf::{FieldsBuilder, Operation, ProtoFieldsPredicate, ProtoValue, ValueMatcher};

const METHOD: &str = "POST";
const URI: &str = "/test.UserService/ListUsers";
const MESSAGE: &str = "test.ListUsersRequest";

mod happy_path {
    use super::*;

    #[tokio::test]
    async fn fields_match_returns_cacheable() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Eq(ProtoValue::String("admin".into())))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn multiple_fields_all_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(5))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Eq(ProtoValue::String("admin".into())))
            .field("page", Operation::Gt(ProtoValue::Int(0)))
            .field("page_size", Operation::Exists)
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }
}

mod operations {
    use super::*;

    #[tokio::test]
    async fn lt_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(3))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("page", Operation::Lt(ProtoValue::Int(10)))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn lt_no_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(15))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("page", Operation::Lt(ProtoValue::Int(10)))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn gt_no_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(3))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("page", Operation::Gt(ProtoValue::Int(10)))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn not_eq_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("user".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::NotEq(ProtoValue::String("admin".into())))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn not_eq_same_value() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::NotEq(ProtoValue::String("admin".into())))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn contains_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("super_admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Contains("admin".into()))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn contains_no_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("guest".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Contains("admin".into()))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn in_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field(
                "role",
                Operation::In(vec![
                    ProtoValue::String("admin".into()),
                    ProtoValue::String("super".into()),
                ]),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn in_no_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("guest".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field(
                "role",
                Operation::In(vec![
                    ProtoValue::String("admin".into()),
                    ProtoValue::String("super".into()),
                ]),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn regex_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin_123".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field(
                "role",
                Operation::Regex(Regex::new(r"^admin_\d+$").unwrap()),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn regex_no_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("guest".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field(
                "role",
                Operation::Regex(Regex::new(r"^admin_\d+$").unwrap()),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

mod predicate_chain {
    use super::*;

    #[tokio::test]
    async fn field_mismatch_returns_non_cacheable() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("guest".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Eq(ProtoValue::String("admin".into())))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn one_field_fails_makes_all_non_cacheable() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(0))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        // role matches, but page Gt(0) fails because page == 0
        let fields = FieldsBuilder::new()
            .field("role", Operation::Eq(ProtoValue::String("admin".into())))
            .field("page", Operation::Gt(ProtoValue::Int(0)))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn inner_non_cacheable_stays_non_cacheable() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Eq(ProtoValue::String("admin".into())))
            .build();

        // Inner returns NonCacheable (Neutral inverted), fields match, but AND semantics → NonCacheable
        let predicate = request::predicate::<Empty<Bytes>>()
            .not()
            .proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

mod error_handling {
    use super::*;

    #[tokio::test]
    async fn invalid_protobuf_returns_non_cacheable() {
        let desc = TestRequest::new(MESSAGE).message_descriptor();
        let request = request_from_bytes(METHOD, URI, Bytes::from_static(b"this is not protobuf"));

        let fields = FieldsBuilder::new()
            .field("role", Operation::Exists)
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn empty_body_returns_non_cacheable() {
        let desc = TestRequest::new(MESSAGE).message_descriptor();
        let request = request_from_bytes(METHOD, URI, Bytes::new());

        let fields = FieldsBuilder::new()
            .field("role", Operation::Exists)
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        // Empty body decodes to a default message where no fields are set
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn body_is_preserved_after_predicate() {
        let builder = TestRequest::new(MESSAGE)
            .field("page", Value::I32(1))
            .field("page_size", Value::I32(10))
            .field("role", Value::String("admin".into()));
        let original_bytes = builder.build_bytes();
        let (desc, request) = builder.build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Exists)
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        let subject = match result {
            PredicateResult::Cacheable(s) => s,
            PredicateResult::NonCacheable(s) => s,
        };

        let (_, body) = subject.into_parts();
        let collected = body.collect().await.expect("body should be collectible");
        assert_eq!(collected.data, original_bytes);
    }
}

mod repeated_fields {
    use super::*;

    #[tokio::test]
    async fn any_scalar_matches() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field(
                "tags",
                Value::List(vec![
                    Value::String("important".into()),
                    Value::String("urgent".into()),
                ]),
            )
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field(
                "tags",
                Operation::any(ValueMatcher::new(Operation::Eq("important".into()))),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn any_scalar_no_match() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field(
                "tags",
                Value::List(vec![
                    Value::String("normal".into()),
                    Value::String("low".into()),
                ]),
            )
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field(
                "tags",
                Operation::any(ValueMatcher::new(Operation::Eq("important".into()))),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn all_scalar_matches() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field(
                "tags",
                Value::List(vec![
                    Value::String("prefix_one".into()),
                    Value::String("prefix_two".into()),
                ]),
            )
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field(
                "tags",
                Operation::all(ValueMatcher::new(Operation::Contains("prefix".into()))),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn all_empty_list_returns_non_cacheable() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("role", Value::String("admin".into()))
            .build_request(METHOD, URI);

        // tags is empty (not set) → All should return false
        let fields = FieldsBuilder::new()
            .field("tags", Operation::all(ValueMatcher::new(Operation::Exists)))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    const RESPONSE_MESSAGE: &str = "test.ListUsersResponse";
    const RESPONSE_URI: &str = "/test.UserService/ListUsers";

    fn make_user(name: &str, age: i32, role: i32) -> Value {
        let pool = common::test_pool();
        let user_desc = pool.get_message("test.User").unwrap();
        let mut user = prost_reflect::DynamicMessage::new(user_desc);
        let name_fd = user.descriptor().get_field_by_name("name").unwrap();
        let age_fd = user.descriptor().get_field_by_name("age").unwrap();
        let role_fd = user.descriptor().get_field_by_name("role").unwrap();
        user.set_field(&name_fd, Value::String(name.into()));
        user.set_field(&age_fd, Value::I32(age));
        user.set_field(&role_fd, Value::EnumNumber(role));
        Value::Message(user)
    }

    #[tokio::test]
    async fn any_message_single_field() {
        let (desc, request) = TestRequest::new(RESPONSE_MESSAGE)
            .field(
                "users",
                Value::List(vec![make_user("alice", 30, 1), make_user("bob", 25, 2)]),
            )
            .field("total_count", Value::I32(2))
            .build_request(METHOD, RESPONSE_URI);

        let fields = FieldsBuilder::new()
            .field(
                "users",
                Operation::any(
                    FieldsBuilder::new()
                        .field("name", Operation::Eq("alice".into()))
                        .build(),
                ),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn any_message_compound_no_match() {
        let (desc, request) = TestRequest::new(RESPONSE_MESSAGE)
            .field(
                "users",
                Value::List(vec![
                    make_user("alice", 30, 1), // ADMIN
                    make_user("bob", 17, 2),   // USER, too young
                ]),
            )
            .field("total_count", Value::I32(2))
            .build_request(METHOD, RESPONSE_URI);

        // Any user where name = "bob" AND age > 20 → should NOT match (bob is 17)
        let fields = FieldsBuilder::new()
            .field(
                "users",
                Operation::any(
                    FieldsBuilder::new()
                        .field("name", Operation::Eq("bob".into()))
                        .field("age", Operation::Gt(ProtoValue::Int(20)))
                        .build(),
                ),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn any_message_compound_match() {
        let (desc, request) = TestRequest::new(RESPONSE_MESSAGE)
            .field(
                "users",
                Value::List(vec![make_user("alice", 30, 1), make_user("bob", 25, 2)]),
            )
            .field("total_count", Value::I32(2))
            .build_request(METHOD, RESPONSE_URI);

        // Any user where name = "alice" AND age > 20 → should match
        let fields = FieldsBuilder::new()
            .field(
                "users",
                Operation::any(
                    FieldsBuilder::new()
                        .field("name", Operation::Eq("alice".into()))
                        .field("age", Operation::Gt(ProtoValue::Int(20)))
                        .build(),
                ),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn all_message_matches() {
        let (desc, request) = TestRequest::new(RESPONSE_MESSAGE)
            .field(
                "users",
                Value::List(vec![make_user("alice", 30, 1), make_user("bob", 25, 2)]),
            )
            .field("total_count", Value::I32(2))
            .build_request(METHOD, RESPONSE_URI);

        // All users have age > 18
        let fields = FieldsBuilder::new()
            .field(
                "users",
                Operation::all(
                    FieldsBuilder::new()
                        .field("age", Operation::Gt(ProtoValue::Int(18)))
                        .build(),
                ),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn all_message_one_fails() {
        let (desc, request) = TestRequest::new(RESPONSE_MESSAGE)
            .field(
                "users",
                Value::List(vec![
                    make_user("alice", 30, 1),
                    make_user("bob", 15, 2), // too young
                ]),
            )
            .field("total_count", Value::I32(2))
            .build_request(METHOD, RESPONSE_URI);

        // All users must have age > 18 → bob fails
        let fields = FieldsBuilder::new()
            .field(
                "users",
                Operation::all(
                    FieldsBuilder::new()
                        .field("age", Operation::Gt(ProtoValue::Int(18)))
                        .build(),
                ),
            )
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

mod map_fields {
    use super::*;

    #[tokio::test]
    async fn path_access() {
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

        let fields = FieldsBuilder::new()
            .field("metadata.env", Operation::Eq("production".into()))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn missing_key() {
        let (desc, request) = TestRequest::new(MESSAGE)
            .field("role", Value::String("admin".into()))
            .field(
                "metadata",
                Value::Map(HashMap::from([(
                    MapKey::String("env".into()),
                    Value::String("production".into()),
                )])),
            )
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("metadata.missing", Operation::Exists)
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

mod enum_fields {
    use super::*;

    #[tokio::test]
    async fn match_by_name() {
        let pool = common::test_pool();
        let user_desc = pool.get_message("test.User").unwrap();
        let mut user = prost_reflect::DynamicMessage::new(user_desc.clone());

        let name_fd = user.descriptor().get_field_by_name("name").unwrap();
        let role_fd = user.descriptor().get_field_by_name("role").unwrap();
        user.set_field(&name_fd, Value::String("alice".into()));
        user.set_field(&role_fd, Value::EnumNumber(1)); // ADMIN = 1

        use prost::Message;
        let body_bytes = Bytes::from(user.encode_to_vec());
        let request = request_from_bytes(METHOD, URI, body_bytes);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Eq(ProtoValue::Enum("ADMIN".into())))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(user_desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn no_match_by_name() {
        let pool = common::test_pool();
        let user_desc = pool.get_message("test.User").unwrap();
        let mut user = prost_reflect::DynamicMessage::new(user_desc.clone());

        let name_fd = user.descriptor().get_field_by_name("name").unwrap();
        let role_fd = user.descriptor().get_field_by_name("role").unwrap();
        user.set_field(&name_fd, Value::String("bob".into()));
        user.set_field(&role_fd, Value::EnumNumber(2)); // USER = 2

        use prost::Message;
        let body_bytes = Bytes::from(user.encode_to_vec());
        let request = request_from_bytes(METHOD, URI, body_bytes);

        let fields = FieldsBuilder::new()
            .field("role", Operation::Eq(ProtoValue::Enum("ADMIN".into())))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(user_desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

mod nested_fields {
    use super::*;

    fn make_user_with_address(name: &str, age: i32, role: i32, city: &str, country: &str) -> Value {
        let pool = common::test_pool();
        let user_desc = pool.get_message("test.User").unwrap();
        let addr_desc = pool.get_message("test.Address").unwrap();

        let mut addr = prost_reflect::DynamicMessage::new(addr_desc);
        let city_fd = addr.descriptor().get_field_by_name("city").unwrap();
        let country_fd = addr.descriptor().get_field_by_name("country").unwrap();
        addr.set_field(&city_fd, Value::String(city.into()));
        addr.set_field(&country_fd, Value::String(country.into()));

        let mut user = prost_reflect::DynamicMessage::new(user_desc);
        let name_fd = user.descriptor().get_field_by_name("name").unwrap();
        let age_fd = user.descriptor().get_field_by_name("age").unwrap();
        let role_fd = user.descriptor().get_field_by_name("role").unwrap();
        let addr_fd = user.descriptor().get_field_by_name("address").unwrap();
        user.set_field(&name_fd, Value::String(name.into()));
        user.set_field(&age_fd, Value::I32(age));
        user.set_field(&role_fd, Value::EnumNumber(role));
        user.set_field(&addr_fd, Value::Message(addr));

        Value::Message(user)
    }

    #[tokio::test]
    async fn dotted_path_match() {
        let pool = common::test_pool();
        let user_desc = pool.get_message("test.User").unwrap();

        let Value::Message(user) = make_user_with_address("alice", 30, 1, "NYC", "US") else {
            panic!("expected Message");
        };

        use prost::Message;
        let body_bytes = Bytes::from(user.encode_to_vec());
        let request = request_from_bytes(METHOD, URI, body_bytes);

        let fields = FieldsBuilder::new()
            .field("address.city", Operation::Eq("NYC".into()))
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(user_desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn dotted_path_missing() {
        let pool = common::test_pool();
        let user_desc = pool.get_message("test.User").unwrap();
        let mut user = prost_reflect::DynamicMessage::new(user_desc.clone());

        // Set only name, no address
        let name_fd = user.descriptor().get_field_by_name("name").unwrap();
        user.set_field(&name_fd, Value::String("alice".into()));

        use prost::Message;
        let body_bytes = Bytes::from(user.encode_to_vec());
        let request = request_from_bytes(METHOD, URI, body_bytes);

        let fields = FieldsBuilder::new()
            .field("address.city", Operation::Exists)
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(user_desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

mod bool_fields {
    use super::*;

    const GET_USER_MESSAGE: &str = "test.GetUserRequest";

    #[tokio::test]
    async fn exists_true() {
        let (desc, request) = TestRequest::new(GET_USER_MESSAGE)
            .field("user_id", Value::String("123".into()))
            .field("include_details", Value::Bool(true))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("include_details", Operation::Exists)
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn exists_false_is_default() {
        let (desc, request) = TestRequest::new(GET_USER_MESSAGE)
            .field("user_id", Value::String("123".into()))
            .field("include_details", Value::Bool(false))
            .build_request(METHOD, URI);

        let fields = FieldsBuilder::new()
            .field("include_details", Operation::Exists)
            .build();

        let predicate = request::predicate::<Empty<Bytes>>().proto_fields(desc, fields);
        let result = predicate.check(request).await;

        // false is the default value for bool → Exists fails
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}
