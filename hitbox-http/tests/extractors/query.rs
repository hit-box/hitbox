use hitbox::Extractor;
use hitbox_http::extractors::{query::QueryExtractor, NeutralExtractor};
use hitbox_http::CacheableHttpRequest;
use http::Request;
use hyper::Body;

#[tokio::test]
async fn test_request_query_extractor_some() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?key=value")
        .build()
        .unwrap();
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
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
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
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
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query("cars".to_owned());
    let parts = extractor.get(request).await;
    dbg!(parts);
}
