use bytes::Bytes;
use hitbox::Extractor;
use hitbox_http::CacheableHttpRequest;
use hitbox_http::extractors::{NeutralExtractor, path::PathExtractor};
use http::Request;
use http_body_util::Empty;

#[tokio::test]
async fn test_request_path_extractor_some() {
    let request = Request::builder()
        .uri("/users/42/books/24/")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().path("/users/{user_id}/books/{book_id}/");
    let parts = extractor.get(request).await;
    dbg!(parts);
}
