use bytes::Bytes;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_core::EvalContext;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::request::HeaderPredicate;
use hitbox_http::predicates::request::header::Operation;
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::{HeaderName, HeaderValue, Request};
use http_body_util::Empty;
use regex::Regex;

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
        let prediction = predicate.check(request, &EvalContext::new()).await;
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
        let prediction = predicate.check(request, &EvalContext::new()).await;
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
        let prediction = predicate.check(request, &EvalContext::new()).await;
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
        let prediction = predicate.check(request, &EvalContext::new()).await;
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
        let prediction = predicate.check(request, &EvalContext::new()).await;
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
        let prediction = predicate.check(request, &EvalContext::new()).await;
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
        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod contains_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let request = Request::builder()
            .header("content-type", "application/json; charset=utf-8")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let predicate = NeutralRequestPredicate::new()
            .header(Operation::contains("content-type".parse().unwrap(), "json"));
        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let request = Request::builder()
            .header("content-type", "text/html")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let predicate = NeutralRequestPredicate::new()
            .header(Operation::contains("content-type".parse().unwrap(), "json"));
        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod regex_tests {
    use super::*;

    #[tokio::test]
    async fn test_positive() {
        let request = Request::builder()
            .header("accept", "application/vnd.api+json; version=3")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let predicate = NeutralRequestPredicate::new().header(Operation::Regex(
            "accept".parse().unwrap(),
            Regex::new(r"version=\d+").unwrap(),
        ));
        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_negative() {
        let request = Request::builder()
            .header("accept", "text/html")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let predicate = NeutralRequestPredicate::new().header(Operation::Regex(
            "accept".parse().unwrap(),
            Regex::new(r"version=\d+").unwrap(),
        ));
        let prediction = predicate.check(request, &EvalContext::new()).await;
        assert!(matches!(prediction, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod constructor_and_from_tests {
    use super::*;

    fn request_with_header(name: &str, value: &str) -> CacheableHttpRequest<Empty<Bytes>> {
        let request = Request::builder()
            .header(name, value)
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap();
        CacheableHttpRequest::from_request(request)
    }

    #[tokio::test]
    async fn test_constructors() {
        // eq
        let predicate = NeutralRequestPredicate::new()
            .header(Operation::eq("x-test".parse().unwrap(), "test-value").unwrap());
        let prediction = predicate
            .check(
                request_with_header("x-test", "test-value"),
                &EvalContext::new(),
            )
            .await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));

        // exist
        let predicate =
            NeutralRequestPredicate::new().header(Operation::exist("x-test".parse().unwrap()));
        let prediction = predicate
            .check(request_with_header("x-test", "any"), &EvalContext::new())
            .await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));

        // any
        let predicate = NeutralRequestPredicate::new().header(Operation::any(
            "x-test".parse().unwrap(),
            vec!["val-a".parse().unwrap(), "val-b".parse().unwrap()],
        ));
        let prediction = predicate
            .check(request_with_header("x-test", "val-b"), &EvalContext::new())
            .await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));

        // regex
        let predicate = NeutralRequestPredicate::new()
            .header(Operation::regex("x-version".parse().unwrap(), r"v\d+\.\d+").unwrap());
        let prediction = predicate
            .check(
                request_with_header("x-version", "v2.1"),
                &EvalContext::new(),
            )
            .await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_from_conversions() {
        // From<HeaderName> (exist check)
        let name: HeaderName = "authorization".parse().unwrap();
        let predicate = NeutralRequestPredicate::new().header(Operation::from(name));
        let prediction = predicate
            .check(
                request_with_header("authorization", "Bearer xyz"),
                &EvalContext::new(),
            )
            .await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));

        // From<(HeaderName, HeaderValue)> (eq check)
        let name: HeaderName = "x-test".parse().unwrap();
        let value: HeaderValue = "exact-value".parse().unwrap();
        let predicate = NeutralRequestPredicate::new().header(Operation::from((name, value)));
        let prediction = predicate
            .check(
                request_with_header("x-test", "exact-value"),
                &EvalContext::new(),
            )
            .await;
        assert!(matches!(prediction, PredicateResult::Cacheable(_)));
    }
}
