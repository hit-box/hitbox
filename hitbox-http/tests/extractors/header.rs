use bytes::Bytes;
use hitbox::Extractor;
use hitbox_http::extractors::{NeutralExtractor, header::HeaderExtractor};
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::Request;
use http_body_util::Empty;

#[tokio::test]
async fn test_request_header_extractor_some() {
    let request = Request::builder()
        .header("x-test", "test-value")
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
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
