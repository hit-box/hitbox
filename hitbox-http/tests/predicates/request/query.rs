use hitbox::predicates::{Operation, Predicate};
use hitbox_http::predicates::query::{QsValue, QueryPredicate};
use hitbox_http::predicates::NeutralPredicate;
use hitbox_http::CacheableHttpRequest;
use http::Request;
use hyper::Body;

#[tokio::test]
async fn test_request_query_predicates_positive() {
    let path = "/path/?name=value";
    let request = CacheableHttpRequest::from_request(
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    );
    let predicate = NeutralPredicate::new().query("name".to_owned(), "value".to_owned());
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
    let predicate = NeutralPredicate::new().query("name".to_owned(), "value".to_owned());
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
    let predicate = NeutralPredicate::new().query("name".to_owned(), "value".to_owned());
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::NonCacheable(_)
    ));
}
