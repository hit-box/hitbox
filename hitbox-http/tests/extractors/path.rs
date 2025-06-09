use hitbox::Extractor;
use hitbox_http::extractors::{path::PathExtractor, NeutralExtractor};
use hitbox_http::CacheableHttpRequest;
use http::Request;
use http_body_util::combinators::UnsyncBoxBody;
use hitbox_http::FromBytes;
use bytes::Bytes;

#[tokio::test]
async fn test_request_path_extractor_some() {
    let request = Request::builder()
        .uri("/users/42/books/24/")
        .body(UnsyncBoxBody::<Bytes, Box<dyn std::error::Error + Send + Sync>>::from_bytes(Bytes::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().path("/users/{user_id}/books/{book_id}/");
    let parts = extractor.get(request).await;
    dbg!(parts);
}
