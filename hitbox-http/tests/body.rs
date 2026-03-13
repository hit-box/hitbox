use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_core::EvalContext;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::request::BodyPredicate;
use hitbox_http::predicates::request::body::{JqExpression, JqOperation, Operation};
use hitbox_http::{BufferedBody, CacheableHttpRequest};
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
        let request = Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let filter = JqExpression::compile(".field").unwrap();
        let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::Eq("test-value".into()),
        });

        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let filter = JqExpression::compile(".field").unwrap();
        let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::Eq("wrong-value".into()),
        });

        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_field_not_found() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
            filter: JqExpression::compile(".wrong_field").unwrap(),
            operation: JqOperation::Eq("test-value".into()),
        });

        let prediction = predicate.check(request, &EvalContext::new()).await;
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
        let request = Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
            filter: JqExpression::compile(".field").unwrap(),
            operation: JqOperation::Exist,
        });

        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"other_field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
            filter: JqExpression::compile(".field").unwrap(),
            operation: JqOperation::Exist,
        });

        let prediction = predicate.check(request, &EvalContext::new()).await;
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
        let request = Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let values = vec!["value-1".to_owned(), "test-value".to_owned()];
        let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
            filter: JqExpression::compile(".field").unwrap(),
            operation: JqOperation::In(values.into_iter().map(|v| v.into()).collect()),
        });

        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"field":"wrong-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let request = Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);

        let values = vec!["value-1".to_owned(), "test-value".to_owned()];
        let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
            filter: JqExpression::compile(".field").unwrap(),
            operation: JqOperation::In(values.into_iter().map(|v| v.into()).collect()),
        });

        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[tokio::test]
async fn test_request_body_predicates_positive_basic() {
    let json_body = r#"{"inner":{"field_one":"value_one","field_two":"value_two"}}"#;
    let body = http_body_util::Full::new(Bytes::from(json_body));
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap(),
    );

    let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
        filter: JqExpression::compile(".inner.field_one").unwrap(),
        operation: JqOperation::Eq("value_one".into()),
    });

    let prediction = predicate.check(request, &EvalContext::new()).await;
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
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap(),
    );

    let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
        filter: JqExpression::compile(".[1].key").unwrap(),
        operation: JqOperation::Eq("my-key-01".into()),
    });

    let prediction = predicate.check(request, &EvalContext::new()).await;
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
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap(),
    );

    let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
        filter: JqExpression::compile(".[].key").unwrap(),
        operation: JqOperation::Eq(json!(["my-key-00", "my-key-01", "my-key-02"])),
    });

    let prediction = predicate.check(request, &EvalContext::new()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}
