use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::conditions::{NotPredicate, OrPredicate};
use hitbox_http::predicates::request::header;
use hitbox_http::predicates::request::query;
use hitbox_http::predicates::request::{
    HeaderPredicate, MethodPredicate, PathPredicate, QueryPredicate,
};
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::Request;
use http_body_util::Empty;

#[tokio::test]
async fn test_conditions_or_cacheable() {
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .method("GET")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let neutral_predicate = NeutralRequestPredicate::new();
    let correct_predicate = NeutralRequestPredicate::new().method(http::Method::GET);
    let wrong_predicate = NeutralRequestPredicate::new().method(http::Method::POST);
    let prediction = neutral_predicate
        .or(correct_predicate, wrong_predicate)
        .check(request)
        .await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_conditions_or_noncacheable_base() {
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .method("GET")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let wrong_base_predicate = NeutralRequestPredicate::new().method(http::Method::PATCH);
    let correct_predicate = NeutralRequestPredicate::new().method(http::Method::GET);
    let wrong_predicate = NeutralRequestPredicate::new().method(http::Method::POST);
    let prediction = wrong_base_predicate
        .or(correct_predicate, wrong_predicate)
        .check(request)
        .await;
    assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
}

#[tokio::test]
async fn test_conditions_or_noncacheable() {
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .method("GET")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let neutral_predicate = NeutralRequestPredicate::new();
    let wrong_predicate_one = NeutralRequestPredicate::new().method(http::Method::DELETE);
    let wrong_predicate_two = NeutralRequestPredicate::new().method(http::Method::POST);
    let prediction = neutral_predicate
        .or(wrong_predicate_one, wrong_predicate_two)
        .check(request)
        .await;
    assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
}

#[tokio::test]
async fn test_conditions_not() {
    let path = "/path/to/resource/?one=two&name=value";
    let expression = "/path/to/resource/wrong/";
    let request = CacheableHttpRequest::from_request(
        Request::builder()
            .header("x-test", "test-value")
            .uri(path)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let correct_query_predicate = NeutralRequestPredicate::<Empty<Bytes>>::new()
        .query(query::Operation::Eq("name".to_owned(), "value".to_owned()));
    let wrong_path_predicate =
        NeutralRequestPredicate::<Empty<Bytes>>::new().path(expression.into());
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
