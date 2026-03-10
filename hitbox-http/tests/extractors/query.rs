use bytes::Bytes;
use hitbox::Extractor;
use hitbox_http::extractors::NeutralExtractor;
use hitbox_http::extractors::query::{NameSelector, QueryConfig, QueryExtractor, ValueExtractor};
use hitbox_http::extractors::transform::Transform;
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::Request;
use http_body_util::Empty;

#[tokio::test]
async fn test_request_query_extractor_some() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?key=value")
        .build()
        .unwrap();
    let request = Request::builder()
        .uri(uri)
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query("key");
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let query_part = key_parts.iter().find(|p| p.key() == "key").unwrap();
    assert_eq!(query_part.value(), Some("value"));
}

#[tokio::test]
async fn test_request_query_extractor_none() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?key=value")
        .build()
        .unwrap();
    let request = Request::builder()
        .uri(uri)
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query("non-existent-key");
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    assert!(key_parts.is_empty());
}

#[tokio::test]
async fn test_request_query_extractor_multiple() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?cars[]=Saab&cars[]=Audi")
        .build()
        .unwrap();
    let request = Request::builder()
        .uri(uri)
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query("cars");
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let car_parts: Vec<_> = key_parts.iter().filter(|p| p.key() == "cars").collect();
    assert_eq!(car_parts.len(), 2);
}

#[tokio::test]
async fn test_request_query_extractor_from_string() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?page=5")
        .build()
        .unwrap();
    let request = Request::builder()
        .uri(uri)
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query(String::from("page"));
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let page_part = key_parts.iter().find(|p| p.key() == "page").unwrap();
    assert_eq!(page_part.value(), Some("5"));
}

#[tokio::test]
async fn test_request_query_extractor_starts_with() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?filter_name=alice&filter_role=admin&page=1")
        .build()
        .unwrap();
    let request = Request::builder()
        .uri(uri)
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor =
        NeutralExtractor::new().query(QueryConfig::name(NameSelector::starts("filter_")));
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    // Should match filter_name and filter_role, not page
    assert_eq!(key_parts.len(), 2);
    assert!(key_parts.iter().any(|p| p.key() == "filter_name"));
    assert!(key_parts.iter().any(|p| p.key() == "filter_role"));
}

#[tokio::test]
async fn test_request_query_extractor_with_regex_value() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?version=v3.2.1")
        .build()
        .unwrap();
    let request = Request::builder()
        .uri(uri)
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().query(
        QueryConfig::name(NameSelector::exact("version"))
            .value(ValueExtractor::Regex(regex::Regex::new(r"v(\d+)").unwrap())),
    );
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let version_part = key_parts.iter().find(|p| p.key() == "version").unwrap();
    assert_eq!(version_part.value(), Some("3"));
}

#[tokio::test]
async fn test_request_query_extractor_with_transform() {
    let uri = http::uri::Uri::builder()
        .path_and_query("test-path?search=HeLLo")
        .build()
        .unwrap();
    let request = Request::builder()
        .uri(uri)
        .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new()
        .query(QueryConfig::name(NameSelector::exact("search")).transform(Transform::Lowercase));
    let parts = extractor.get(request).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let search_part = key_parts.iter().find(|p| p.key() == "search").unwrap();
    assert_eq!(search_part.value(), Some("hello"));
}
