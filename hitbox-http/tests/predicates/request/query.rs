use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_core::EvalContext;
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
    let prediction = predicate.check(request, &mut EvalContext::new()).await;
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
    let prediction = predicate.check(request, &mut EvalContext::new()).await;
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
    let prediction = predicate.check(request, &mut EvalContext::new()).await;
    assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
}

#[tokio::test]
async fn test_request_query_from_conversions() {
    // From<&str> creates an Exist operation
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri("/path/?name=value")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let op: query::Operation = "name".into();
    let predicate = NeutralRequestPredicate::new().query(op);
    let prediction = predicate.check(request, &mut EvalContext::new()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));

    // From<(&str, &str)> creates an Eq operation
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri("/path/?format=json")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let op: query::Operation = ("format", "json").into();
    let predicate = NeutralRequestPredicate::new().query(op);
    let prediction = predicate.check(request, &mut EvalContext::new()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_request_query_constructors() {
    // eq + exist chained
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri("/path/?page=3&limit=10")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralRequestPredicate::new()
        .query(query::Operation::eq("page", "3"))
        .query(query::Operation::exist("limit"));
    let prediction = predicate.check(request, &mut EvalContext::new()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));

    // any constructor
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .uri("/path/?sort=desc")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralRequestPredicate::new().query(query::Operation::any(
        "sort",
        vec!["asc".to_owned(), "desc".to_owned()],
    ));
    let prediction = predicate.check(request, &mut EvalContext::new()).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}
