use bytes::Bytes;
use hitbox::Extractor;
use hitbox_http::extractors::NeutralExtractor;
use hitbox_http::extractors::header::{
    HeaderConfig, HeaderExtractor, NameSelector, ValueExtractor,
};
use hitbox_http::extractors::transform::Transform;
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
    let extractor = NeutralExtractor::new().header("x-test");
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let header_part = key_parts.iter().find(|p| p.key() == "x-test").unwrap();
    assert_eq!(header_part.value(), Some("test-value"));
}

#[tokio::test]
async fn test_request_header_extractor_from_string() {
    let request = Request::builder()
        .header("x-api-key", "secret123")
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().header(String::from("x-api-key"));
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let header_part = key_parts.iter().find(|p| p.key() == "x-api-key").unwrap();
    assert_eq!(header_part.value(), Some("secret123"));
}

#[tokio::test]
async fn test_request_header_extractor_starts_with() {
    let request = Request::builder()
        .header("x-custom-one", "val1")
        .header("x-custom-two", "val2")
        .header("x-other", "val3")
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor =
        NeutralExtractor::new().header(HeaderConfig::name(NameSelector::starts("x-custom")));
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    // Should match x-custom-one and x-custom-two, but not x-other
    assert_eq!(key_parts.len(), 2);
    assert!(key_parts.iter().any(|p| p.key() == "x-custom-one"));
    assert!(key_parts.iter().any(|p| p.key() == "x-custom-two"));
}

#[tokio::test]
async fn test_request_header_extractor_with_regex_value() {
    let request = Request::builder()
        .header("accept", "application/json; version=3")
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor =
        NeutralExtractor::new().header(HeaderConfig::name(NameSelector::exact("accept")).value(
            ValueExtractor::Regex(regex::Regex::new(r"version=(\d+)").unwrap()),
        ));
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let accept_part = key_parts.iter().find(|p| p.key() == "accept").unwrap();
    assert_eq!(accept_part.value(), Some("3"));
}

#[tokio::test]
async fn test_request_header_extractor_with_transform() {
    let request = Request::builder()
        .header("x-token", "AbCdEf")
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new()
        .header(HeaderConfig::name(NameSelector::exact("x-token")).transform(Transform::Lowercase));
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let token_part = key_parts.iter().find(|p| p.key() == "x-token").unwrap();
    assert_eq!(token_part.value(), Some("abcdef"));
}
