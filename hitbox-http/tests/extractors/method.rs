use bytes::Bytes;
use hitbox::Extractor;
use hitbox_http::extractors::{NeutralExtractor, method::MethodExtractor};
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::{Method, Request};
use http_body_util::Empty;

#[tokio::test]
async fn test_request_method_extractor_some() {
    let request = Request::builder()
        .uri("/users/42/books/24/")
        .method(Method::POST)
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().method();
    let parts = extractor.get(request).await;
    dbg!(parts);
}
