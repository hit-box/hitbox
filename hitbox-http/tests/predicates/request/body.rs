use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::request::body::Operation;
use hitbox_http::predicates::request::BodyPredicate;
use hitbox_http::predicates::request::ParsingType;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::CacheableHttpRequest;
use http::Request;
use hyper::Body as HttpBody;
use serde_json::json;

#[cfg(test)]
mod eq_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let request = Request::builder().body(HttpBody::from(json_body)).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Eq("test-value".into()),
        );

        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"field":"test-value"}"#;
        let request = Request::builder().body(HttpBody::from(json_body)).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Eq("wrong-value".into()),
        );

        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_field_not_found() {
        let json_body = r#"{"field":"test-value"}"#;
        let request = Request::builder().body(HttpBody::from(json_body)).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".wrong_field".to_owned(),
            Operation::Eq("test-value".into()),
        );

        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod exist_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let request = Request::builder().body(HttpBody::from(json_body)).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Exist,
        );

        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"other_field":"test-value"}"#;
        let request = Request::builder().body(HttpBody::from(json_body)).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Exist,
        );

        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod in_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let request = Request::builder().body(HttpBody::from(json_body)).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let values = vec!["value-1".to_owned(), "test-value".to_owned()];
        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::In(values.into_iter().map(|v| v.into()).collect()),
        );

        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"field":"wrong-value"}"#;
        let request = Request::builder().body(HttpBody::from(json_body)).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let values = vec!["value-1".to_owned(), "test-value".to_owned()];
        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::In(values.into_iter().map(|v| v.into()).collect()),
        );

        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[tokio::test]
async fn test_request_body_predicates_positive_basic() {
    let json_body = r#"{"inner":{"field_one":"value_one","field_two":"value_two"}}"#;
    let request = CacheableHttpRequest::from_request(
        Request::builder().body(HttpBody::from(json_body)).unwrap(),
    );

    let predicate = NeutralRequestPredicate::new().body(
        ParsingType::Jq,
        ".inner.field_one".to_owned(),
        Operation::Eq("value_one".into()),
    );

    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_request_body_predicates_positive_array() {
    let json_body = r#"
    [
        {"key": "my-key-00", "value": "my-value-00"},
        {"key": "my-key-01", "value": "my-value-01"}
    ]"#;
    let request = CacheableHttpRequest::from_request(
        Request::builder().body(HttpBody::from(json_body)).unwrap(),
    );

    let predicate = NeutralRequestPredicate::new().body(
        ParsingType::Jq,
        ".[1].key".to_owned(),
        Operation::Eq("my-key-01".into()),
    );

    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_request_body_predicates_positive_multiple_value() {
    let json_body = r#"
    [
        {"key": "my-key-00", "value": "my-value-00"},
        {"key": "my-key-01", "value": "my-value-01"},
        {"key": "my-key-02", "value": "my-value-02"}
    ]"#;
    let request = CacheableHttpRequest::from_request(
        Request::builder().body(HttpBody::from(json_body)).unwrap(),
    );

    let predicate = NeutralRequestPredicate::new().body(
        ParsingType::Jq,
        ".[].key".to_owned(),
        Operation::Eq(json!(["my-key-00", "my-key-01", "my-key-02"])),
    );

    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[cfg(test)]
mod protobuf_tests {
    use super::*;
    use prost_reflect::prost::Message;
    use prost_reflect::{DescriptorPool, DynamicMessage, Value as ReflectValue};
    use std::fs;

    const TEST_PROTO: &str = r#"
        syntax = "proto3";

        package test;

        message TestMessage {
            int32 foo = 1;
        }
    "#;

    #[tokio::test]
    async fn test_protobuf_body_predicate() {
        // Create a proto file
        fs::write("test.proto", TEST_PROTO).unwrap();

        // Create a descriptor pool with our test message
        let descriptor_set = protox::compile(["test.proto"], ["."]).unwrap();
        let pool = DescriptorPool::from_file_descriptor_set(descriptor_set).unwrap();
        let descriptor = pool.get_message_by_name("test.TestMessage").unwrap();

        // Create a dynamic message
        let mut dynamic_msg = DynamicMessage::new(descriptor.clone());
        dynamic_msg.set_field_by_name("foo", ReflectValue::I32(42));

        // Create a request with the protobuf message
        let encoded = dynamic_msg.encode_to_vec();
        let body = HttpBody::from(encoded);
        let request = Request::builder().body(body).unwrap();
        let cacheable_request = CacheableHttpRequest::from_request(request);

        // Create the predicate
        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::ProtoBuf(descriptor),
            ".foo".to_string(),
            Operation::Eq(serde_json::json!(42)),
        );

        // Test the predicate
        let result = predicate.check(cacheable_request).await;
        match result {
            PredicateResult::Cacheable(_) => (),
            _ => panic!("Expected Cacheable result"),
        }

        // Clean up
        fs::remove_file("test.proto").unwrap();
    }
}
