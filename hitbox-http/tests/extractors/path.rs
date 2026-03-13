use bytes::Bytes;
use hitbox::Extractor;
use hitbox_core::EvalContext;
use hitbox_http::extractors::{NeutralExtractor, path::PathExtractor};
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::Request;
use http_body_util::Empty;

#[tokio::test]
async fn test_request_path_extractor_some() {
    let request = Request::builder()
        .uri("/users/42/books/24/")
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().path("/users/{user_id}/books/{book_id}/");
    let parts = extractor.get(request, &EvalContext::new()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let user_part = key_parts.iter().find(|p| p.key() == "user_id").unwrap();
    assert_eq!(user_part.value(), Some("42"));
    let book_part = key_parts.iter().find(|p| p.key() == "book_id").unwrap();
    assert_eq!(book_part.value(), Some("24"));
}

#[tokio::test]
async fn test_request_path_extractor_from_owned_string_ref() {
    let request = Request::builder()
        .uri("/api/v2/items")
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let pattern = String::from("/api/{version}/items");
    let extractor = NeutralExtractor::new().path(&pattern);
    let parts = extractor.get(request, &EvalContext::new()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let version_part = key_parts.iter().find(|p| p.key() == "version").unwrap();
    assert_eq!(version_part.value(), Some("v2"));
}
