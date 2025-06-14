use bytes::Bytes;
use hitbox::Extractor;
use hitbox_http::extractors::{
    header::HeaderExtractor, method::MethodExtractor, path::PathExtractor, NeutralExtractor,
};
use hitbox_http::CacheableHttpRequest;
use http::{Method, Request};
use http_body_util::Empty;

#[tokio::test]
async fn test_request_multiple_extractor_some() {
    let request = Request::builder()
        .uri("/users/42/books/24/")
        .method(Method::PUT)
        .header("X-test", "x-test-value")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new()
        .path("/users/{user_id}/books/{book_id}/")
        .method()
        .header("x-test".to_owned());
    let parts = extractor.get(request).await;
    dbg!(parts);
}
