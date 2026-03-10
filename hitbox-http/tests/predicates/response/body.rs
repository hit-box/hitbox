use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::NeutralResponsePredicate;
use hitbox_http::predicates::response::body::BodyPredicate;
use hitbox_http::predicates::response::{JqExpression, JqOperation, Operation, PlainOperation};
use hitbox_http::{BufferedBody, CacheableHttpResponse};
use http::Response;
use serde_json::json;

#[cfg(test)]
mod jq_eq_tests {
    use super::*;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let filter = JqExpression::compile(".field").unwrap();
        let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::Eq(json!("test-value")),
        });

        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let filter = JqExpression::compile(".field").unwrap();
        let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::Eq(json!("wrong-value")),
        });

        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_field_not_found() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let filter = JqExpression::compile(".wrong_field").unwrap();
        let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::Eq(json!("test-value")),
        });

        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod jq_exist_tests {
    use super::*;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let filter = JqExpression::compile(".field").unwrap();
        let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::Exist,
        });

        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"other_field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let filter = JqExpression::compile(".field").unwrap();
        let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::Exist,
        });

        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod jq_in_tests {
    use super::*;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let filter = JqExpression::compile(".field").unwrap();
        let values = vec![json!("value-1"), json!("test-value")];
        let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::In(values),
        });

        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let json_body = r#"{"field":"wrong-value"}"#;
        let body = Full::new(Bytes::from(json_body));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let filter = JqExpression::compile(".field").unwrap();
        let values = vec![json!("value-1"), json!("test-value")];
        let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
            filter,
            operation: JqOperation::In(values),
        });

        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[tokio::test]
async fn test_response_body_jq_nested_field() {
    let json_body = r#"{"inner":{"field_one":"value_one","field_two":"value_two"}}"#;
    let body = http_body_util::Full::new(Bytes::from(json_body));
    let response = CacheableHttpResponse::from_response(
        Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap(),
    );

    let filter = JqExpression::compile(".inner.field_one").unwrap();
    let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
        filter,
        operation: JqOperation::Eq(json!("value_one")),
    });

    let prediction = predicate.check(response, &mut Default::default()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_response_body_jq_array_index() {
    let json_body = r#"
    [
        {"key": "my-key-00", "value": "my-value-00"},
        {"key": "my-key-01", "value": "my-value-01"}
    ]"#;
    let body = http_body_util::Full::new(Bytes::from(json_body));
    let response = CacheableHttpResponse::from_response(
        Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap(),
    );

    let filter = JqExpression::compile(".[1].key").unwrap();
    let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
        filter,
        operation: JqOperation::Eq(json!("my-key-01")),
    });

    let prediction = predicate.check(response, &mut Default::default()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_response_body_jq_array_map() {
    let json_body = r#"
    [
        {"key": "my-key-00", "value": "my-value-00"},
        {"key": "my-key-01", "value": "my-value-01"},
        {"key": "my-key-02", "value": "my-value-02"}
    ]"#;
    let body = http_body_util::Full::new(Bytes::from(json_body));
    let response = CacheableHttpResponse::from_response(
        Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap(),
    );

    let filter = JqExpression::compile(".[].key").unwrap();
    let predicate = NeutralResponsePredicate::new().body(Operation::Jq {
        filter,
        operation: JqOperation::Eq(json!(["my-key-00", "my-key-01", "my-key-02"])),
    });

    let prediction = predicate.check(response, &mut Default::default()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[cfg(test)]
mod constructor_tests {
    use super::*;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_limit_constructor() {
        // cacheable (within limit)
        let body = Full::new(Bytes::from("small body"));
        let response = CacheableHttpResponse::from_response(
            Response::builder()
                .body(BufferedBody::Passthrough(body))
                .unwrap(),
        );
        let predicate = NeutralResponsePredicate::new().body(Operation::limit(1024));
        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));

        // non-cacheable (exceeds limit)
        let body = Full::new(Bytes::from("a".repeat(100)));
        let response = CacheableHttpResponse::from_response(
            Response::builder()
                .body(BufferedBody::Passthrough(body))
                .unwrap(),
        );
        let predicate = NeutralResponsePredicate::new().body(Operation::limit(10));
        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_plain_and_jq_constructors() {
        // plain constructor
        let body = Full::new(Bytes::from(r#"{"success":true}"#));
        let response = CacheableHttpResponse::from_response(
            Response::builder()
                .body(BufferedBody::Passthrough(body))
                .unwrap(),
        );
        let predicate = NeutralResponsePredicate::new()
            .body(Operation::plain(PlainOperation::Contains("success".into())));
        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));

        // jq constructor
        let body = Full::new(Bytes::from(r#"{"status":"ok"}"#));
        let response = CacheableHttpResponse::from_response(
            Response::builder()
                .body(BufferedBody::Passthrough(body))
                .unwrap(),
        );
        let predicate = NeutralResponsePredicate::new()
            .body(Operation::jq(".status", JqOperation::Eq(json!("ok"))).unwrap());
        let prediction = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }
}
