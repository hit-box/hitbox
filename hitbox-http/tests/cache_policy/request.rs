use hitbox::{
    cache::{CachePolicy, CacheableRequest},
    predicates::Operation,
};
use hitbox_http::{predicates::header::Header, CacheableHttpRequest};
use http::Request;
use hyper::Body;

#[tokio::test]
async fn test_cache_policy() {
    let request = Request::builder()
        .header("header-key", "header-value")
        .body(Body::empty())
        .unwrap();
    let cacheable = CacheableHttpRequest::from_request(request);
    let predicates = vec![Header {
        name: "header-key".to_owned(),
        value: "header-value".to_owned(),
        operation: Operation::Eq,
    }];
    let policy = cacheable.cache_policy(&predicates).await;
    match &policy {
        CachePolicy::Cacheable(_) => println!("CachePolicy::Cacheable"),
        CachePolicy::NonCacheable(_) => println!("CachePolicy::NonCacheable"),
    };
    assert!(matches!(policy, CachePolicy::Cacheable(_)));
}
