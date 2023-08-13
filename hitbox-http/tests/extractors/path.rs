use hitbox::cache::Extractor;
use hitbox_http::extractors::{path::PathExtractor, NeutralExtractor};
use hitbox_http::CacheableHttpRequest;
use http::Request;
use hyper::Body;

#[tokio::test]
async fn test_request_path_extractor_some() {
    let request = Request::builder()
        .uri("/users/42/books/24/")
        .body(Body::empty())
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().path("/users/{user_id}/books/{book_id}/");
    let parts = extractor.get(request).await;
    dbg!(parts);
}
