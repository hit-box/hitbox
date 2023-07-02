use hitbox::predicates::{Operation, Predicate};
use hitbox_http::predicates::path::Path;
use hitbox_http::CacheableHttpRequest;
use http::Request;
use hyper::Body;

#[tokio::test]
async fn test_request_path_predicates_full_match() {
    let path = "/path/to/resource/";
    let expression = "/path/to/resource/";
    let request = CacheableHttpRequest::from_request(
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    );
    let predicate = Path::new(expression.to_string());
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::Cacheable(_)
    ));
}

#[tokio::test]
async fn test_request_path_predicates_use_expression() {
    let path = "/path/to/resource/";
    let expression = "/path/{arg}/resource/";
    let request = CacheableHttpRequest::from_request(
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    );
    let predicate = Path::new(expression.to_string());
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::Cacheable(_)
    ));
}

#[tokio::test]
async fn test_request_path_predicates_non_match() {
    let path = "/path/42";
    let expression = "/path/34";
    let request = CacheableHttpRequest::from_request(
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    );
    let predicate = Path::new(expression.to_string());
    let prediction = predicate.check(request).await;
    assert!(matches!(
        prediction,
        hitbox::predicates::PredicateResult::NonCacheable(_)
    ));
}
