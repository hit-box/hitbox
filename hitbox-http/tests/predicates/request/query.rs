use hitbox::predicates::{Operation, Predicate};
use hitbox_http::predicates::query::{QsValue, Query};
use hitbox_http::CacheableHttpRequest;
use http::Request;
use hyper::Body;

#[tokio::test]
async fn test_request_query_predicates_positive() {
    let path = "/path/?name=value";
    let request = CacheableHttpRequest::from_request(
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    );
    let predicate = Query {
        name: String::from("name"),
        value: QsValue::Scalar(String::from("value")),
        operation: Operation::Eq,
    };
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::Cacheable(_)
    ));
}

#[tokio::test]
async fn test_request_query_predicates_multiple() {
    let path = "/path/?one=two&name=value";
    let request = CacheableHttpRequest::from_request(
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    );
    let predicate = Query {
        name: String::from("name"),
        value: QsValue::Scalar(String::from("value")),
        operation: Operation::Eq,
    };
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::Cacheable(_)
    ));
}

#[tokio::test]
async fn test_request_query_predicates_negative() {
    let path = "/path/?one=two&three=four";
    let request = CacheableHttpRequest::from_request(
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    );
    let predicate = Query {
        name: String::from("name"),
        value: QsValue::Scalar(String::from("value")),
        operation: Operation::Eq,
    };
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::NonCacheable(_)
    ));
}
