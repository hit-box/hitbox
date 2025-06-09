use hitbox::Extractor;
use hitbox_http::extractors::{query::QueryExtractor, NeutralExtractor};
use hitbox_http::CacheableHttpRequest;
use http::Request;
use http_body_util::combinators::UnsyncBoxBody;
use hitbox_http::FromBytes;
use bytes::Bytes;

#[tokio::test]
async fn test_request_query_extractor_some() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?key=value")
        .build()
        .unwrap();
    let request = Request::builder().uri(uri).body(UnsyncBoxBody::<Bytes, Box<dyn std::error::Error + Send + Sync>>::from_bytes(Bytes::new())).unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query("key".to_owned());
    let parts = extractor.get(request).await;
    dbg!(parts);
}

#[tokio::test]
async fn test_request_query_extractor_none() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?key=value")
        .build()
        .unwrap();
    let request = Request::builder().uri(uri).body(UnsyncBoxBody::<Bytes, Box<dyn std::error::Error + Send + Sync>>::from_bytes(Bytes::new())).unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query("non-existent-key".to_owned());
    let parts = extractor.get(request).await;
    dbg!(parts);
}

#[tokio::test]
async fn test_request_query_extractor_multiple() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?cars[]=Saab&cars[]=Audi")
        .build()
        .unwrap();
    let request = Request::builder().uri(uri).body(UnsyncBoxBody::<Bytes, Box<dyn std::error::Error + Send + Sync>>::from_bytes(Bytes::new())).unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query("cars".to_owned());
    let parts = extractor.get(request).await;
    dbg!(parts);
}
