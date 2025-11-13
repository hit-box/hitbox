use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::{
    BufferedBody, CacheableHttpResponse,
    predicates::{NeutralResponsePredicate, response::StatusCodePredicate},
};
use http::{Response, StatusCode};
use http_body_util::Empty;

#[tokio::test]
async fn test_response_predicates_match() {
    let status = 200;
    let request = CacheableHttpResponse::from_response(
        Response::builder()
            .status(status)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate =
        NeutralResponsePredicate::new().status_code(StatusCode::from_u16(status).unwrap());
    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_response_predicates_not_match() {
    let predicate_status = 200;
    let response_status = 500;
    let request = CacheableHttpResponse::from_response(
        Response::builder()
            .status(response_status)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );
    let predicate = NeutralResponsePredicate::new()
        .status_code(StatusCode::from_u16(predicate_status).unwrap());
    let prediction = predicate.check(request).await;
    assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
}
