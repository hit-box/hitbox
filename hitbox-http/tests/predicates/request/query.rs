use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::request::QueryPredicate;
use hitbox_http::predicates::request::query;
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::Request;
use http_body_util::Empty;

#[tokio::test]
async fn test_request_query_predicates_positive() {
    let path = "/path/?name=value";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri(path)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralRequestPredicate::new()
        .query(query::Operation::Eq("name".to_owned(), "value".to_owned()));
    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_request_query_predicates_multiple() {
    let path = "/path/?one=two&name=value";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri(path)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralRequestPredicate::new().query(query::Operation::In(
        "name".to_owned(),
        vec!["value".to_owned(), "second-value".to_owned()],
    ));
    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_request_query_predicates_negative() {
    let path = "/path/?one=two&three=four";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri(path)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralRequestPredicate::new().query(query::Operation::Eq(
        "name".to_owned(),
        "wrong-value".to_owned(),
    ));
    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
}
