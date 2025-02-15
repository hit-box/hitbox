use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::request::body::Operation;
use hitbox_http::predicates::request::BodyPredicate;
use hitbox_http::predicates::request::ParsingType;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::CacheableHttpRequest;
use http::Request;
use hyper::Body;
use serde_json::json;

#[cfg(test)]
mod eq_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let json_body = r#"{"field":"test-value"}"#;
        let request = Request::builder().body(Body::from(json_body)).unwrap();
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
        let request = Request::builder().body(Body::from(json_body)).unwrap();
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
        let request = Request::builder().body(Body::from(json_body)).unwrap();
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
        let request = Request::builder().body(Body::from(json_body)).unwrap();
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
        let request = Request::builder().body(Body::from(json_body)).unwrap();
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
        let request = Request::builder().body(Body::from(json_body)).unwrap();
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
        let request = Request::builder().body(Body::from(json_body)).unwrap();
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
    let request =
        CacheableHttpRequest::from_request(Request::builder().body(Body::from(json_body)).unwrap());

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
    let request =
        CacheableHttpRequest::from_request(Request::builder().body(Body::from(json_body)).unwrap());

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
    let request =
        CacheableHttpRequest::from_request(Request::builder().body(Body::from(json_body)).unwrap());

    let predicate = NeutralRequestPredicate::new().body(
        ParsingType::Jq,
        ".[].key".to_owned(),
        Operation::Eq(json!(["my-key-00", "my-key-01", "my-key-02"])),
    );

    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}
