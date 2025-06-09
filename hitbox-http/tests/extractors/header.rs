use hitbox::Extractor;
use hitbox_http::extractors::{header::HeaderExtractor, NeutralExtractor};
use hitbox_http::CacheableHttpRequest;
use http::Request;
use http_body_util::combinators::UnsyncBoxBody;
use hitbox_http::FromBytes;
use bytes::Bytes;

#[tokio::test]
async fn test_request_header_extractor_some() {
    let request = Request::builder()
        .header("x-test", "test-value")
        .body(UnsyncBoxBody::<Bytes, Box<dyn std::error::Error + Send + Sync>>::from_bytes(Bytes::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().header("x-test".to_owned());
    let parts = extractor.get(request).await;
    dbg!(parts);
    // assert!(matches!(
    //     prediction,
    //     hitbox::predicates::PredicateResult::Cacheable(_)
    // ));
}
