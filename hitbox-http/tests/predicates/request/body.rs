use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::CacheableHttpRequest;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::request::BodyPredicate;
use hitbox_http::predicates::request::ParsingType;
use hitbox_http::predicates::request::body::Operation;
use http::Request;
use serde_json::json;

#[cfg(test)]
mod eq_tests {
    use super::*;
    use bytes::Bytes;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder().body(body).unwrap();
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
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder().body(body).unwrap();
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
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder().body(body).unwrap();
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
    use http_body_util::Full;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder().body(body).unwrap();
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
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder().body(body).unwrap();
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
    use http_body_util::Full;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder().body(body).unwrap();
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
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder().body(body).unwrap();
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
    let body = http_body_util::Full::new(Bytes::from(json_body));
    let request = CacheableHttpRequest::from_request(Request::builder().body(body).unwrap());

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
    let body = http_body_util::Full::new(Bytes::from(json_body));
    let request = CacheableHttpRequest::from_request(Request::builder().body(body).unwrap());

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
    let body = http_body_util::Full::new(Bytes::from(json_body));
    let request = CacheableHttpRequest::from_request(Request::builder().body(body).unwrap());

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
        let body = http_body_util::Full::new(Bytes::from(encoded));
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

#[cfg(test)]
mod error_handling_tests {
    use super::*;
    use bytes::Bytes;
    use hitbox_http::FromBytes;
    use hitbox_http::FromChunks;
    use http_body_util::BodyExt;
    use std::fmt;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    // Mock error type simulating network/timeout errors
    #[derive(Debug, Clone)]
    struct NetworkError;

    impl fmt::Display for NetworkError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Network error occurred")
        }
    }

    impl std::error::Error for NetworkError {}

    // Mock body that yields some chunks successfully, then an error
    struct ErrorProducingBody {
        chunks: Vec<Result<Bytes, NetworkError>>,
        index: usize,
    }

    impl ErrorProducingBody {
        fn new() -> Self {
            Self {
                chunks: vec![
                    Ok(Bytes::from(r#"{"field":"#)),
                    Ok(Bytes::from(r#""test-value"}"#)),
                    Err(NetworkError),
                ],
                index: 0,
            }
        }
    }

    impl hyper::body::Body for ErrorProducingBody {
        type Data = Bytes;
        type Error = NetworkError;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
            if self.index >= self.chunks.len() {
                return Poll::Ready(None);
            }

            let result = self.chunks[self.index].clone();
            self.index += 1;

            Poll::Ready(Some(result.map(hyper::body::Frame::data)))
        }
    }

    impl FromBytes for ErrorProducingBody {
        fn from_bytes(bytes: Bytes) -> Self {
            Self {
                chunks: vec![Ok(bytes)],
                index: 0,
            }
        }
    }

    impl FromChunks<NetworkError> for ErrorProducingBody {
        fn from_chunks(chunks: Vec<Result<Bytes, NetworkError>>) -> Self {
            Self { chunks, index: 0 }
        }
    }

    #[tokio::test]
    async fn test_body_collection_error_returns_non_cacheable() {
        let body = ErrorProducingBody::new();
        let request = Request::builder().body(body).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Eq("test-value".into()),
        );

        // Should not panic and should return NonCacheable
        let prediction = predicate.check(request).await;
        assert!(
            matches!(prediction, PredicateResult::NonCacheable(_)),
            "Expected NonCacheable when body collection error occurs"
        );
    }

    #[tokio::test]
    async fn test_body_error_preserves_error_in_reconstructed_body() {
        let body = ErrorProducingBody::new();
        let request = Request::builder().body(body).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Eq("test-value".into()),
        );

        let prediction = predicate.check(request).await;

        // Extract the request from the result
        let reconstructed_request = match prediction {
            PredicateResult::NonCacheable(req) => req,
            _ => panic!("Expected NonCacheable result"),
        };

        // Verify the reconstructed body contains the exact same error
        let (_, mut body) = reconstructed_request.into_parts();
        let mut chunks_collected = 0;
        let mut found_error = false;

        while let Some(frame) = body.frame().await {
            match frame {
                Ok(frame) => {
                    if frame.is_data() {
                        chunks_collected += 1;
                    }
                }
                Err(err) => {
                    // Verify it's our NetworkError
                    assert_eq!(
                        err.to_string(),
                        "Network error occurred",
                        "Error message should match the injected NetworkError"
                    );
                    found_error = true;
                    break;
                }
            }
        }

        assert_eq!(chunks_collected, 2, "Should have collected 2 successful chunks before error");
        assert!(found_error, "Reconstructed body should contain the original NetworkError");
    }
}
