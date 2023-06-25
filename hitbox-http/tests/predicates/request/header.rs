use hitbox::predicates::{Operation, Predicate};
use hitbox_http::predicates::header::Header;
use hitbox_http::CacheableHttpRequest;
use http::Request;
use hyper::Body;

#[tokio::test]
async fn test_request_header_predicates_positive() {
    let request = Request::builder()
        .header("x-test", "test-value")
        .body(Body::empty())
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let predicate = Header {
        name: "x-test".to_owned(),
        value: "test-value".to_owned(),
        operation: Operation::Eq,
    };
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::Cacheable(_)
    ));
}

#[tokio::test]
async fn test_request_header_predicates_negative_by_key() {
    let request = Request::builder()
        .header("x-test", "test-value")
        .body(Body::empty())
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let predicate = Header {
        name: "missing".to_owned(),
        value: "test-value".to_owned(),
        operation: Operation::Eq,
    };
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::NonCacheable(_)
    ));
}

#[tokio::test]
async fn test_request_header_predicates_negative_by_value() {
    let request = Request::builder()
        .header("x-test", "test-value")
        .body(Body::empty())
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let predicate = Header {
        name: "x-test".to_owned(),
        value: "missing".to_owned(),
        operation: Operation::Eq,
    };
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::NonCacheable(_)
    ));
}
