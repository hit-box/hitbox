use futures::stream;
use hitbox::{
    cache::{CachePolicy, CacheableRequest},
    predicates::{Operation, Predicate},
};
use hitbox_http::{
    predicates::{body::Body, header::Header},
    CacheableHttpRequest,
};
use http::Request;

#[tokio::test]
async fn test_cache_policy() {
    let request = Request::builder()
        .header("header-key", "header-value")
        .body(hyper::Body::empty())
        .unwrap();
    let cacheable = CacheableHttpRequest::from_request(request);
    let predicates = vec![Header {
        name: "header-key".to_owned(),
        value: "header-value".to_owned(),
        operation: Operation::Eq,
    }];
    let policy = cacheable.cache_policy(&predicates).await;
    assert!(matches!(policy, CachePolicy::Cacheable(_)));
}

#[tokio::test]
async fn test_body_cache_policy() {
    let stream: Vec<Result<_, std::io::Error>> = vec![Ok("12"), Ok("34")];
    let request = Request::builder()
        .header("header-key", "header-value")
        .body(hyper::Body::wrap_stream(stream::iter(stream)))
        .unwrap();
    let cacheable = CacheableHttpRequest::from_request(request);
    let predicates = vec![
        // Box::new(Header {
        //     name: "header-key".to_owned(),
        //     value: "header-value".to_owned(),
        //     operation: Operation::Eq,
        // }),
        Box::new(Body),
    ];
    let policy = cacheable.cache_policy(&predicates).await;
    match &policy {
        CachePolicy::Cacheable(_) => println!("CachePolicy::Cacheable"),
        CachePolicy::NonCacheable(_) => println!("CachePolicy::NonCacheable"),
    };
    assert!(matches!(policy, CachePolicy::Cacheable(_)));
}
