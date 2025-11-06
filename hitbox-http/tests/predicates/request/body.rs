use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::CacheableHttpRequest;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::request::BodyPredicate;
use hitbox_http::predicates::request::ParsingType;
use hitbox_http::predicates::request::body::Operation;
use http::Request;
use serde_json::json;

/// Test utilities for simulating error scenarios
mod test_utils {
    use bytes::Bytes;
    use hitbox_http::FromBytes;
    use http_body::{Body, Frame, SizeHint};
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    /// A body that drops the connection after sending some bytes
    pub struct DroppingBody {
        data: Option<Bytes>,
        fail_after: usize,
        sent: usize,
    }

    impl DroppingBody {
        /// Create a body that will send `fail_after` bytes then return an error
        pub fn new(data: Bytes, fail_after: usize) -> Self {
            Self {
                data: Some(data),
                fail_after,
                sent: 0,
            }
        }
    }

    impl Body for DroppingBody {
        type Data = Bytes;
        type Error = io::Error;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            if let Some(data) = self.data.take() {
                if self.sent >= self.fail_after {
                    // Connection dropped!
                    return Poll::Ready(Some(Err(io::Error::new(
                        io::ErrorKind::ConnectionReset,
                        "connection dropped during body transmission",
                    ))));
                }

                let chunk_size = self.fail_after.min(data.len());
                self.sent += chunk_size;

                if chunk_size < data.len() {
                    // Split the data
                    let chunk = data.slice(0..chunk_size);
                    self.data = Some(data.slice(chunk_size..));
                    Poll::Ready(Some(Ok(Frame::data(chunk))))
                } else {
                    // Send all data, will fail on next poll
                    Poll::Ready(Some(Ok(Frame::data(data))))
                }
            } else if self.sent >= self.fail_after {
                // Already sent data, now fail
                Poll::Ready(Some(Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "connection dropped",
                ))))
            } else {
                Poll::Ready(None)
            }
        }

        fn size_hint(&self) -> SizeHint {
            if let Some(data) = &self.data {
                SizeHint::with_exact(data.len() as u64)
            } else {
                SizeHint::with_exact(0)
            }
        }
    }

    impl FromBytes for DroppingBody {
        fn from_bytes(bytes: Bytes) -> Self {
            Self {
                data: Some(bytes),
                fail_after: usize::MAX, // Won't fail if reconstructed
                sent: 0,
            }
        }
    }

    /// A body that exceeds the size limit
    pub struct OversizedBody {
        size: usize,
        sent: usize,
        chunk_size: usize,
    }

    impl OversizedBody {
        /// Create a body that will send `size` bytes in chunks of `chunk_size`
        pub fn new(size: usize, chunk_size: usize) -> Self {
            Self {
                size,
                sent: 0,
                chunk_size,
            }
        }
    }

    impl Body for OversizedBody {
        type Data = Bytes;
        type Error = io::Error;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            if self.sent >= self.size {
                return Poll::Ready(None);
            }

            let remaining = self.size - self.sent;
            let to_send = remaining.min(self.chunk_size);
            self.sent += to_send;

            let chunk = vec![b'x'; to_send];
            Poll::Ready(Some(Ok(Frame::data(Bytes::from(chunk)))))
        }

        fn size_hint(&self) -> SizeHint {
            let remaining = self.size.saturating_sub(self.sent);
            SizeHint::with_exact(remaining as u64)
        }
    }

    impl FromBytes for OversizedBody {
        fn from_bytes(bytes: Bytes) -> Self {
            Self {
                size: bytes.len(),
                sent: 0,
                chunk_size: bytes.len(),
            }
        }
    }

    /// A body that immediately returns an error
    pub struct ErrorBody {
        error_kind: io::ErrorKind,
        message: String,
    }

    impl ErrorBody {
        pub fn new(error_kind: io::ErrorKind, message: impl Into<String>) -> Self {
            Self {
                error_kind,
                message: message.into(),
            }
        }

        pub fn connection_reset() -> Self {
            Self::new(io::ErrorKind::ConnectionReset, "connection reset by peer")
        }

        pub fn timeout() -> Self {
            Self::new(io::ErrorKind::TimedOut, "operation timed out")
        }
    }

    impl Body for ErrorBody {
        type Data = Bytes;
        type Error = io::Error;

        fn poll_frame(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
            Poll::Ready(Some(Err(io::Error::new(
                self.error_kind,
                self.message.clone(),
            ))))
        }

        fn size_hint(&self) -> SizeHint {
            SizeHint::default()
        }
    }

    impl FromBytes for ErrorBody {
        fn from_bytes(bytes: Bytes) -> Self {
            Self {
                error_kind: io::ErrorKind::Other,
                message: format!("reconstructed from {} bytes", bytes.len()),
            }
        }
    }
}

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
mod error_scenarios {
    use super::*;
    use test_utils::*;

    #[tokio::test]
    #[should_panic(expected = "unwrap")]
    async fn test_connection_dropped_mid_body() {
        // Simulate connection dropping after 50 bytes
        let json_body = r#"{"field":"test-value-that-is-very-long-and-exceeds-50-bytes-easily"}"#;
        let body = DroppingBody::new(Bytes::from(json_body), 50);

        let request = Request::builder().body(body).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Exist,
        );

        // This should panic because we're calling unwrap() on the error
        let _prediction = predicate.check(request).await;
    }

    #[tokio::test]
    #[should_panic(expected = "unwrap")]
    async fn test_connection_reset_immediately() {
        // Connection fails immediately
        let body = ErrorBody::connection_reset();

        let request = Request::builder().body(body).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Exist,
        );

        // Should panic due to unwrap on error
        let _prediction = predicate.check(request).await;
    }

    #[tokio::test]
    #[should_panic(expected = "unwrap")]
    async fn test_timeout_during_collection() {
        // Simulate timeout
        let body = ErrorBody::timeout();

        let request = Request::builder().body(body).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Exist,
        );

        // Should panic due to unwrap on error
        let _prediction = predicate.check(request).await;
    }

    #[tokio::test]
    #[should_panic(expected = "unwrap")]
    async fn test_body_exceeds_size_limit() {
        // Create a 20KB body (exceeds MAX_BODY_SIZE = 10KB)
        let body = OversizedBody::new(20 * 1024, 1024);

        let request = Request::builder().body(body).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Exist,
        );

        // Should panic due to size exceeded error + unwrap
        let _prediction = predicate.check(request).await;
    }

    #[tokio::test]
    #[should_panic(expected = "unwrap")]
    async fn test_partial_data_before_drop() {
        // Send some valid JSON start, then drop
        let partial_json = r#"{"field":"test-va"#; // Incomplete JSON
        let body = DroppingBody::new(Bytes::from(partial_json), 10);

        let request = Request::builder().body(body).unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(
            ParsingType::Jq,
            ".field".to_owned(),
            Operation::Exist,
        );

        // Should panic - either collection error or parse error + unwrap
        let _prediction = predicate.check(request).await;
    }
}
