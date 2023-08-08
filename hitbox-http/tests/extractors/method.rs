use hitbox::cache::Extractor;
use hitbox_http::extractors::{method::MethodExtractor, NeutralExtractor};
use hitbox_http::CacheableHttpRequest;
use http::{Method, Request};
use hyper::Body;

#[tokio::test]
async fn test_request_method_extractor_some() {
    let request = Request::builder()
        .uri("/users/42/books/24/")
        .method(Method::POST)
        .body(Body::empty())
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().method();
    let parts = extractor.get(request).await;
    dbg!(parts);
}
