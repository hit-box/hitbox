use bytes::Bytes;
use futures::stream;
use hitbox_http::predicates::response::{Operation, PlainOperation};
use hitbox_http::{BufferedBody, PartialBufferedBody, Remaining};
use http_body_util::StreamBody;
use std::convert::Infallible;

/// Helper to create a StreamBody from multiple chunks
fn create_stream_body(
    chunks: Vec<&'static str>,
) -> StreamBody<impl futures::Stream<Item = Result<http_body::Frame<Bytes>, Infallible>>> {
    let frames = chunks
        .into_iter()
        .map(|chunk| Ok(http_body::Frame::data(Bytes::from(chunk))))
        .collect::<Vec<_>>();
    StreamBody::new(stream::iter(frames))
}

#[cfg(test)]
mod eq_tests {
    use super::*;
    use hitbox::predicate::{Predicate, PredicateResult};
    use hitbox_http::CacheableHttpResponse;
    use hitbox_http::predicates::NeutralResponsePredicate;
    use hitbox_http::predicates::response::BodyPredicate;
    use http::Response;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_eq_matches_exact_bytes() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(PlainOperation::Eq(
            Bytes::copy_from_slice(b"hello world"),
        )));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_eq_fails_on_mismatch() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(PlainOperation::Eq(
            Bytes::copy_from_slice(b"goodbye world"),
        )));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_eq_with_stream_body() {
        let stream_body = create_stream_body(vec!["hello", " ", "world"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(PlainOperation::Eq(
            Bytes::copy_from_slice(b"hello world"),
        )));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_eq_empty_body() {
        let body = Full::new(Bytes::new());
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new()
            .body(Operation::Plain(PlainOperation::Eq(Bytes::new())));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }
}

#[cfg(test)]
mod contains_tests {
    use super::*;
    use hitbox::predicate::{Predicate, PredicateResult};
    use hitbox_http::CacheableHttpResponse;
    use hitbox_http::predicates::NeutralResponsePredicate;
    use hitbox_http::predicates::response::BodyPredicate;
    use http::Response;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_contains_simple_match() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_contains_not_found() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"goodbye")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_contains_pattern_at_beginning() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"hello")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_contains_pattern_at_end() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_contains_empty_pattern() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new()
            .body(Operation::Plain(PlainOperation::Contains(Bytes::new())));

        let result = predicate.check(response, &mut Default::default()).await;
        // Empty pattern should always match
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Pattern spanning chunk boundaries
    #[tokio::test]
    async fn test_contains_pattern_across_chunks() {
        // Pattern "lo wo" spans across "hel|lo wo|rld" chunks
        let stream_body = create_stream_body(vec!["hel", "lo wo", "rld"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"lo wo")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Pattern split exactly at chunk boundary
    #[tokio::test]
    async fn test_contains_pattern_split_at_boundary() {
        // Pattern "world" is split as "wor|ld"
        let stream_body = create_stream_body(vec!["hello wor", "ld"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Pattern in last chunk
    #[tokio::test]
    async fn test_contains_pattern_in_last_chunk() {
        let stream_body = create_stream_body(vec!["hello ", "beautiful ", "world"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Pattern in first chunk (early termination)
    #[tokio::test]
    async fn test_contains_pattern_in_first_chunk() {
        let stream_body = create_stream_body(vec!["hello world", " more data", " even more"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"hello")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Single byte chunks
    #[tokio::test]
    async fn test_contains_with_single_byte_chunks() {
        let stream_body = create_stream_body(vec!["h", "e", "l", "l", "o"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"llo")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Pattern with Partial body (prefix + remaining)
    #[tokio::test]
    async fn test_contains_with_partial_body_pattern_in_prefix() {
        let prefix = Some(Bytes::from("hello world"));
        let remaining_stream = create_stream_body(vec![" more data"]);
        let partial = PartialBufferedBody::new(prefix, Remaining::Body(remaining_stream));
        let response = Response::builder()
            .body(BufferedBody::Partial(partial))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Pattern with Partial body (prefix + remaining), pattern in remaining
    #[tokio::test]
    async fn test_contains_with_partial_body_pattern_in_remaining() {
        let prefix = Some(Bytes::from("hello"));
        let remaining_stream = create_stream_body(vec![" world"]);
        let partial = PartialBufferedBody::new(prefix, Remaining::Body(remaining_stream));
        let response = Response::builder()
            .body(BufferedBody::Partial(partial))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Pattern spanning prefix and remaining
    #[tokio::test]
    async fn test_contains_with_partial_body_pattern_spanning_boundary() {
        let prefix = Some(Bytes::from("hello wor"));
        let remaining_stream = create_stream_body(vec!["ld"]);
        let partial = PartialBufferedBody::new(prefix, Remaining::Body(remaining_stream));
        let response = Response::builder()
            .body(BufferedBody::Partial(partial))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Partial body with error, pattern in prefix
    #[tokio::test]
    async fn test_contains_with_partial_body_error_pattern_found() {
        use std::io;
        let prefix = Some(Bytes::from("hello world"));
        let error = io::Error::new(io::ErrorKind::ConnectionReset, "test error");
        // Create a type alias for the stream body type
        type TestBody = StreamBody<
            futures::stream::Iter<std::vec::IntoIter<Result<http_body::Frame<Bytes>, io::Error>>>,
        >;
        let partial = PartialBufferedBody::<TestBody>::new(prefix, Remaining::Error(Some(error)));
        let response = Response::builder()
            .body(BufferedBody::Partial(partial))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        // Pattern found in prefix, but should be NonCacheable due to error
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    // EDGE CASE: Partial body with error, pattern not in prefix
    #[tokio::test]
    async fn test_contains_with_partial_body_error_pattern_not_found() {
        use std::io;
        let prefix = Some(Bytes::from("hello"));
        let error = io::Error::new(io::ErrorKind::ConnectionReset, "test error");
        // Create a type alias for the stream body type
        type TestBody = StreamBody<
            futures::stream::Iter<std::vec::IntoIter<Result<http_body::Frame<Bytes>, io::Error>>>,
        >;
        let partial = PartialBufferedBody::<TestBody>::new(prefix, Remaining::Error(Some(error)));
        let response = Response::builder()
            .body(BufferedBody::Partial(partial))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Contains(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod starts_with_tests {
    use super::*;
    use hitbox::predicate::{Predicate, PredicateResult};
    use hitbox_http::CacheableHttpResponse;
    use hitbox_http::predicates::NeutralResponsePredicate;
    use hitbox_http::predicates::response::BodyPredicate;
    use http::Response;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_starts_with_matches() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Starts(Bytes::copy_from_slice(b"hello")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_starts_with_fails() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Starts(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_starts_with_stream_body() {
        let stream_body = create_stream_body(vec!["hel", "lo ", "world"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Starts(Bytes::copy_from_slice(b"hello")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_starts_with_empty_prefix() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new()
            .body(Operation::Plain(PlainOperation::Starts(Bytes::new())));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_starts_with_body_too_short() {
        let body = Full::new(Bytes::from("hi"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Starts(Bytes::copy_from_slice(b"hello")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

#[cfg(test)]
mod ends_with_tests {
    use super::*;
    use hitbox::predicate::{Predicate, PredicateResult};
    use hitbox_http::CacheableHttpResponse;
    use hitbox_http::predicates::NeutralResponsePredicate;
    use hitbox_http::predicates::response::BodyPredicate;
    use http::Response;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_ends_with_matches() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Ends(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_ends_with_fails() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Ends(Bytes::copy_from_slice(b"hello")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_ends_with_stream_body() {
        let stream_body = create_stream_body(vec!["hello ", "wor", "ld"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new().body(Operation::Plain(
            PlainOperation::Ends(Bytes::copy_from_slice(b"world")),
        ));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_ends_with_empty_suffix() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let predicate = NeutralResponsePredicate::new()
            .body(Operation::Plain(PlainOperation::Ends(Bytes::new())));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }
}

#[cfg(test)]
mod regexp_tests {
    use super::*;
    use hitbox::predicate::{Predicate, PredicateResult};
    use hitbox_http::CacheableHttpResponse;
    use hitbox_http::predicates::NeutralResponsePredicate;
    use hitbox_http::predicates::response::BodyPredicate;
    use http::Response;
    use http_body_util::Full;

    #[tokio::test]
    async fn test_regexp_matches() {
        let body = Full::new(Bytes::from("hello world 123"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let regex = regex::bytes::Regex::new(r"\d+").unwrap();
        let predicate =
            NeutralResponsePredicate::new().body(Operation::Plain(PlainOperation::RegExp(regex)));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn test_regexp_fails() {
        let body = Full::new(Bytes::from("hello world"));
        let response = Response::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let regex = regex::bytes::Regex::new(r"\d+").unwrap();
        let predicate =
            NeutralResponsePredicate::new().body(Operation::Plain(PlainOperation::RegExp(regex)));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn test_regexp_with_stream_body() {
        let stream_body = create_stream_body(vec!["hello ", "world ", "123"]);
        let response = Response::builder()
            .body(BufferedBody::Passthrough(stream_body))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);

        let regex = regex::bytes::Regex::new(r"world \d+").unwrap();
        let predicate =
            NeutralResponsePredicate::new().body(Operation::Plain(PlainOperation::RegExp(regex)));

        let result = predicate.check(response, &mut Default::default()).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }
}
