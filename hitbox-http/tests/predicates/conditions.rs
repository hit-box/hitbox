use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::conditions::{NotPredicate, OrPredicate};
use hitbox_http::predicates::request::header;
use hitbox_http::predicates::request::query;
use hitbox_http::predicates::request::{HeaderPredicate, PathPredicate, QueryPredicate};
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::CacheableHttpRequest;
use http::Request;
use http_body_util::Empty;

#[tokio::test]
async fn test_conditions_or() {
    let path = "/path/to/resource/?one=two&name=value";
    let expression = "/path/to/resource/wrong/";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .header("x-test", "test-value")
            .uri(path)
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let wrong_query_predicate = NeutralRequestPredicate::new().query(query::Operation::Eq(
        "name".to_owned(),
        "wrong-value".to_owned(),
    ));
    let wrong_path_predicate = NeutralRequestPredicate::new().path(expression.into());
    let correct_header_predicate = NeutralRequestPredicate::new().header(header::Operation::Eq(
        "x-test".parse().unwrap(),
        "test-value".parse().unwrap(),
    ));
    let prediction = wrong_query_predicate
        .or(wrong_path_predicate)
        .or(correct_header_predicate)
        .check(request)
        .await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_conditions_not() {
    let path = "/path/to/resource/?one=two&name=value";
    let expression = "/path/to/resource/wrong/";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .header("x-test", "test-value")
            .uri(path)
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let correct_query_predicate =
        NeutralRequestPredicate::<CacheableHttpRequest<Empty<Bytes>>>::new()
            .query(query::Operation::Eq("name".to_owned(), "value".to_owned()));
    let wrong_path_predicate = NeutralRequestPredicate::<CacheableHttpRequest<Empty<Bytes>>>::new()
        .path(expression.into());
    let wrong_header_predicate = NeutralRequestPredicate::new().header(header::Operation::Eq(
        "x-test".parse().unwrap(),
        "wrong-test-value".parse().unwrap(),
    ));
    let prediction = correct_query_predicate
        .not(wrong_path_predicate)
        .not(wrong_header_predicate)
        .check(request)
        .await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}
