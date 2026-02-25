use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::response::StatusClass;
use hitbox_http::predicates::response::status::Operation as StatusOp;
use hitbox_http::{
    BufferedBody, CacheableHttpResponse,
    predicates::{NeutralResponsePredicate, response::StatusCodePredicate},
};
use http::{Response, StatusCode};
use http_body_util::Empty;

fn response(status: u16) -> CacheableHttpResponse<Empty<Bytes>> {
    CacheableHttpResponse::from_response(
        Response::builder()
            .status(status)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    )
}

#[tokio::test]
async fn test_response_predicates_match() {
    let predicate =
        NeutralResponsePredicate::new().status(StatusOp::eq(StatusCode::from_u16(200).unwrap()));
    let prediction = predicate.check(response(200)).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_response_predicates_not_match() {
    let predicate =
        NeutralResponsePredicate::new().status(StatusOp::eq(StatusCode::from_u16(200).unwrap()));
    let prediction = predicate.check(response(500)).await;
    assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
}

#[tokio::test]
async fn test_status_class_matching() {
    for (status, class) in [
        (100, StatusClass::Informational),
        (301, StatusClass::Redirect),
        (404, StatusClass::ClientError),
        (503, StatusClass::ServerError),
    ] {
        let predicate = NeutralResponsePredicate::new().status(class);
        let prediction = predicate.check(response(status)).await;
        assert!(
            matches!(prediction, PredicateResult::Cacheable(_)),
            "expected Cacheable for status {status}"
        );
    }
}

#[tokio::test]
async fn test_status_class_mismatch() {
    let predicate = NeutralResponsePredicate::new().status(StatusClass::ClientError);
    let prediction = predicate.check(response(200)).await;
    assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
}

#[tokio::test]
async fn test_status_from_conversions() {
    // From StatusCode
    let op: StatusOp = StatusCode::OK.into();
    let predicate = NeutralResponsePredicate::new().status(op);
    let prediction = predicate.check(response(200)).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));

    // From StatusClass
    let op: StatusOp = StatusClass::Success.into();
    let predicate = NeutralResponsePredicate::new().status(op);
    let prediction = predicate.check(response(201)).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_status_range() {
    let predicate = NeutralResponsePredicate::new().status(StatusOp::range(
        StatusCode::OK,
        StatusCode::from_u16(299).unwrap(),
    ));
    let prediction = predicate.check(response(204)).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_status_any() {
    let predicate = NeutralResponsePredicate::new()
        .status(StatusOp::any(vec![StatusCode::OK, StatusCode::CREATED]));
    let prediction = predicate.check(response(201)).await;
    assert!(matches!(prediction, PredicateResult::Cacheable(_)));
}
