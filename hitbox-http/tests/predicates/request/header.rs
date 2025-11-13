use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::request::HeaderPredicate;
use hitbox_http::predicates::request::header::Operation;
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::{HeaderName, HeaderValue, Request};
use http_body_util::Empty;

#[cfg(test)]
mod eq_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let request = Request::builder()
            .header("x-test", "test-value")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let name: HeaderName = "x-test".to_string().parse().unwrap();
        let value: HeaderValue = "test-value".to_string().parse().unwrap();
        let predicate = NeutralRequestPredicate::new().header(Operation::Eq(name, value));
        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let request = Request::builder()
            .header("x-test", "test-value")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let name: HeaderName = "x-test".to_string().parse().unwrap();
        let value: HeaderValue = "wrong-value".to_string().parse().unwrap();
        let predicate = NeutralRequestPredicate::new().header(Operation::Eq(name, value));
        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_name_not_found() {
        let request = Request::builder()
            .header("x-test", "test-value")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let name: HeaderName = "wrong-name".to_string().parse().unwrap();
        let value: HeaderValue = "test-value".to_string().parse().unwrap();
        let predicate = NeutralRequestPredicate::new().header(Operation::Eq(name, value));
        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod exist_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let request = Request::builder()
            .header("x-test", "test-value")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let name: HeaderName = "x-test".to_string().parse().unwrap();
        let predicate = NeutralRequestPredicate::new().header(Operation::Exist(name));
        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let request = Request::builder()
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let name: HeaderName = "x-test".to_string().parse().unwrap();
        let predicate = NeutralRequestPredicate::new().header(Operation::Exist(name));
        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod in_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let request = Request::builder()
            .header("x-test", "test-value")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let name: HeaderName = "x-test".to_string().parse().unwrap();
        let values = vec![
            "value-1".to_string().parse().unwrap(),
            "test-value".to_string().parse().unwrap(),
        ];
        let predicate = NeutralRequestPredicate::new().header(Operation::In(name, values));
        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let request = Request::builder()
            .header("x-test", "wrong-value")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let name: HeaderName = "x-test".to_string().parse().unwrap();
        let values = vec![
            "value-1".to_string().parse().unwrap(),
            "test-value".to_string().parse().unwrap(),
        ];
        let predicate = NeutralRequestPredicate::new().header(Operation::In(name, values));
        let prediction = predicate.check(request).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}
