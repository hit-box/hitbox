use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::request::{PathPredicate, path};
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::Request;
use http_body_util::Empty;

#[tokio::test]
async fn test_request_path_predicates_full_match() {
    let path = "/path/to/resource/";
    let expression = "/path/to/resource/";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri(path)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralRequestPredicate::new().path(path::Operation::pattern(expression));
    let prediction = predicate.check(request, &mut Default::default()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_request_path_predicates_use_expression() {
    let path = "/path/to/resource/";
    let expression = "/path/{arg}/resource/";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri(path)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralRequestPredicate::new().path(path::Operation::pattern(expression));
    let prediction = predicate.check(request, &mut Default::default()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_request_path_predicates_non_match() {
    let path = "/path/42";
    let expression = "/path/34";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri(path)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralRequestPredicate::new().path(path::Operation::pattern(expression));
    let prediction = predicate.check(request, &mut Default::default()).await;
    assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
}

#[tokio::test]
async fn test_request_path_from_conversions() {
    // From<&str>
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri("/api/users/42")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let op: path::Operation = "/api/users/{id}".into();
    let predicate = NeutralRequestPredicate::new().path(op);
    let prediction = predicate.check(request, &mut Default::default()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));

    // From<String>
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri("/api/users/42")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let op: path::Operation = String::from("/api/users/{id}").into();
    let predicate = NeutralRequestPredicate::new().path(op);
    let prediction = predicate.check(request, &mut Default::default()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}
