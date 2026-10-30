//! Comprehensive tests for `BufferedBody` across all HTTP protocol scenarios.
//!
//! Test cases are organized by the 4 possible body/trailer combinations
//! that occur in real HTTP traffic, then tested through both direct method
//! calls (foundations) and predicate operations (real usage patterns).
//!
//! Predicates exercise different internal code paths:
//! - `Eq` / `Ends` / `RegExp` → `collect()` (full body consumption)
//! - `Contains` → `streaming_search` (chunk-by-chunk with early exit)
//! - `Starts` → `collect_exact` (partial consumption)

use bytes::Bytes;
use futures::stream;
use hitbox::predicate::PredicateResult;
use hitbox_http::predicates::response::PlainOperation;
use hitbox_http::{BufferedBody, CollectExactResult, PartialBufferedBody, Remaining};
use http::HeaderMap;
use http_body::Body;
use http_body_util::{BodyExt, Full, StreamBody};
use std::convert::Infallible;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_trailers() -> HeaderMap {
    let mut map = HeaderMap::new();
    map.insert("grpc-status", "0".parse().unwrap());
    map.insert("grpc-message", "OK".parse().unwrap());
    map
}

type InfallibleStream = StreamBody<
    futures::stream::Iter<std::vec::IntoIter<Result<http_body::Frame<Bytes>, Infallible>>>,
>;

fn make_stream(chunks: &[&str]) -> InfallibleStream {
    let frames: Vec<_> = chunks
        .iter()
        .map(|c| Ok(http_body::Frame::data(Bytes::from(c.to_string()))))
        .collect();
    StreamBody::new(stream::iter(frames))
}

fn make_stream_with_trailers(chunks: &[&str], trailers: HeaderMap) -> InfallibleStream {
    let mut frames: Vec<Result<http_body::Frame<Bytes>, Infallible>> = chunks
        .iter()
        .map(|c| Ok(http_body::Frame::data(Bytes::from(c.to_string()))))
        .collect();
    frames.push(Ok(http_body::Frame::trailers(trailers)));
    StreamBody::new(stream::iter(frames))
}

type IoErrorStream = StreamBody<
    futures::stream::Iter<std::vec::IntoIter<Result<http_body::Frame<Bytes>, std::io::Error>>>,
>;

fn make_stream_with_error(chunks: &[&str], error: std::io::Error) -> IoErrorStream {
    let mut frames: Vec<Result<http_body::Frame<Bytes>, std::io::Error>> = chunks
        .iter()
        .map(|c| Ok(http_body::Frame::data(Bytes::from(c.to_string()))))
        .collect();
    frames.push(Err(error));
    StreamBody::new(stream::iter(frames))
}

// ===========================================================================
// Case 1: No body, no trailers
// ===========================================================================

/// HTTP/1.1 or HTTP/2 empty responses (`204 No Content`, `304 Not Modified`)
/// or bodyless requests (`GET`, `DELETE`).
///
/// `BufferedBody::Complete { data: None, trailers: None }`
mod case1_no_body_no_trailers {
    use super::*;

    // -- Direct method tests --

    #[tokio::test]
    async fn poll_frame_returns_none_immediately() {
        let mut body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: None,
        };
        assert!(body.frame().await.is_none());
    }

    #[tokio::test]
    async fn is_end_stream_true() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: None,
        };
        assert!(body.is_end_stream());
    }

    #[tokio::test]
    async fn size_hint_zero() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: None,
        };
        let hint = body.size_hint();
        assert_eq!(hint.lower(), 0);
        assert_eq!(hint.upper(), Some(0));
    }

    #[tokio::test]
    async fn collect_returns_empty() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: None,
        };
        let collected = body.collect().await.unwrap();
        assert!(collected.data.is_empty());
        assert!(collected.trailers.is_none());
    }

    // -- Predicate tests --

    #[tokio::test]
    async fn eq_empty_matches() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: None,
        };
        let result = PlainOperation::Eq(Bytes::new()).check(body).await;
        let PredicateResult::Cacheable(body) = result else {
            panic!("Expected Cacheable, got NonCacheable");
        };
        let collected = body.collect().await.unwrap();
        assert!(collected.data.is_empty());
        assert!(collected.trailers.is_none());
    }

    #[tokio::test]
    async fn contains_non_empty_pattern_does_not_match() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: None,
        };
        let result = PlainOperation::Contains(Bytes::from("hello"))
            .check(body)
            .await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn starts_non_empty_prefix_does_not_match() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: None,
        };
        let result = PlainOperation::Starts(Bytes::from("hello"))
            .check(body)
            .await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }
}

// ===========================================================================
// Case 2: Body present, no trailers
// ===========================================================================

/// The most common HTTP scenario — HTTP/1.1 or HTTP/2 responses carrying
/// a JSON payload, HTML page, file download, etc. No trailing headers.
mod case2_body_no_trailers {
    use super::*;

    /// Body already fully buffered in memory (e.g., cache hit).
    mod complete {
        use super::*;

        #[tokio::test]
        async fn poll_frame_yields_data_then_none() {
            let mut body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello")),
                trailers: None,
            };
            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("hello"));
            assert!(body.frame().await.is_none());
        }

        #[tokio::test]
        async fn is_end_stream_with_data_pending() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello")),
                trailers: None,
            };
            assert!(!body.is_end_stream());
        }

        #[tokio::test]
        async fn size_hint_exact() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello")),
                trailers: None,
            };
            let hint = body.size_hint();
            assert_eq!(hint.lower(), 5);
            assert_eq!(hint.upper(), Some(5));
        }

        // -- Through predicates --

        #[tokio::test]
        async fn eq_match_preserves_body() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello world")),
                trailers: None,
            };
            let result = PlainOperation::Eq(Bytes::from("hello world"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            assert!(collected.trailers.is_none());
        }

        #[tokio::test]
        async fn eq_mismatch() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello world")),
                trailers: None,
            };
            let result = PlainOperation::Eq(Bytes::from("goodbye")).check(body).await;
            assert!(matches!(result, PredicateResult::NonCacheable(_)));
        }

        #[tokio::test]
        async fn contains_match_preserves_body() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello world")),
                trailers: None,
            };
            let result = PlainOperation::Contains(Bytes::from("world"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
        }

        #[tokio::test]
        async fn starts_match_preserves_body() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello world")),
                trailers: None,
            };
            let result = PlainOperation::Starts(Bytes::from("hello"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
        }
    }

    /// Partially consumed body — a predicate already read some bytes,
    /// the rest is still streaming.
    mod partial {
        use super::*;

        #[tokio::test]
        async fn poll_frame_yields_prefix_then_remaining() {
            let stream = make_stream(&["remaining"]);
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("prefix-")), Remaining::Body(stream));
            let mut body = BufferedBody::Partial(partial);

            // First frame: prefix
            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("prefix-"));

            // Second frame: remaining stream chunk
            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("remaining"));

            assert!(body.frame().await.is_none());
        }

        #[tokio::test]
        async fn poll_frame_no_prefix_with_body() {
            let stream = make_stream(&["data"]);
            let partial = PartialBufferedBody::new(None, Remaining::Body(stream));
            let mut body = BufferedBody::Partial(partial);

            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("data"));
            assert!(body.frame().await.is_none());
        }

        #[tokio::test]
        async fn poll_frame_error_remaining() {
            let error = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset");
            let partial: PartialBufferedBody<IoErrorStream> =
                PartialBufferedBody::new(Some(Bytes::from("data")), Remaining::Error(Some(error)));
            let mut body = BufferedBody::Partial(partial);

            // First: prefix data
            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("data"));

            // Second: error
            let result = body.frame().await.unwrap();
            assert!(result.is_err());

            // Third: end
            assert!(body.frame().await.is_none());
        }

        #[tokio::test]
        async fn is_end_stream_with_prefix() {
            let stream = make_stream(&["data"]);
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("prefix")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);
            assert!(!body.is_end_stream());
        }

        #[tokio::test]
        async fn is_end_stream_error_consumed() {
            let partial: PartialBufferedBody<IoErrorStream> =
                PartialBufferedBody::new(None, Remaining::Error(None));
            let body = BufferedBody::Partial(partial);
            // Error already consumed, no prefix — end of stream
            assert!(body.is_end_stream());
        }

        #[tokio::test]
        async fn size_hint_prefix_plus_remaining() {
            let inner = Full::new(Bytes::from("remaining"));
            let partial = PartialBufferedBody::new(
                Some(Bytes::from("prefix")), // 6 bytes
                Remaining::Body(inner),      // 9 bytes
            );
            let body = BufferedBody::Partial(partial);
            let hint = body.size_hint();
            assert_eq!(hint.lower(), 15); // 6 + 9
            assert_eq!(hint.upper(), Some(15));
        }

        #[tokio::test]
        async fn size_hint_error_remaining() {
            let partial: PartialBufferedBody<Full<Bytes>> =
                PartialBufferedBody::new(Some(Bytes::from("prefix")), Remaining::Error(None));
            let body = BufferedBody::Partial(partial);
            let hint = body.size_hint();
            assert_eq!(hint.lower(), 6);
            assert_eq!(hint.upper(), Some(6));
        }

        // -- Through predicates --

        #[tokio::test]
        async fn eq_match_on_partial_body() {
            let stream = make_stream(&[" world"]);
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("hello")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);

            let result = PlainOperation::Eq(Bytes::from("hello world"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
        }

        #[tokio::test]
        async fn contains_in_prefix() {
            let stream = make_stream(&[" more"]);
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("hello world")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);

            let result = PlainOperation::Contains(Bytes::from("world"))
                .check(body)
                .await;
            assert!(matches!(result, PredicateResult::Cacheable(_)));
        }

        #[tokio::test]
        async fn contains_in_remaining() {
            let stream = make_stream(&[" world"]);
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("hello")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);

            let result = PlainOperation::Contains(Bytes::from("world"))
                .check(body)
                .await;
            assert!(matches!(result, PredicateResult::Cacheable(_)));
        }

        #[tokio::test]
        async fn contains_spanning_prefix_and_remaining() {
            let stream = make_stream(&["ld"]);
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("hello wor")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);

            let result = PlainOperation::Contains(Bytes::from("world"))
                .check(body)
                .await;
            assert!(matches!(result, PredicateResult::Cacheable(_)));
        }

        #[tokio::test]
        async fn starts_match_on_partial() {
            let stream = make_stream(&[" world"]);
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("hello")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);

            let result = PlainOperation::Starts(Bytes::from("hello"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
        }
    }

    /// Untouched body — forwarded directly from upstream.
    mod passthrough {
        use super::*;

        #[tokio::test]
        async fn poll_frame_multi_chunk() {
            let mut body = BufferedBody::Passthrough(make_stream(&["a", "b", "c"]));

            let f1 = body.frame().await.unwrap().unwrap().into_data().unwrap();
            let f2 = body.frame().await.unwrap().unwrap().into_data().unwrap();
            let f3 = body.frame().await.unwrap().unwrap().into_data().unwrap();
            assert_eq!(
                (f1.as_ref(), f2.as_ref(), f3.as_ref()),
                (b"a".as_ref(), b"b".as_ref(), b"c".as_ref())
            );
            assert!(body.frame().await.is_none());
        }

        #[tokio::test]
        async fn poll_frame_with_error() {
            let error = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "reset");
            let mut body = BufferedBody::Passthrough(make_stream_with_error(&["data"], error));

            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("data"));

            let result = body.frame().await.unwrap();
            assert!(result.is_err());
        }

        // -- Through predicates --

        #[tokio::test]
        async fn eq_on_multi_chunk_stream() {
            let body = BufferedBody::Passthrough(make_stream(&["hello", " ", "world"]));
            let result = PlainOperation::Eq(Bytes::from("hello world"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            assert!(collected.trailers.is_none());
        }

        #[tokio::test]
        async fn contains_pattern_in_first_chunk_early_exit() {
            let body = BufferedBody::Passthrough(make_stream(&["hello world", " more", " data"]));
            let result = PlainOperation::Contains(Bytes::from("hello"))
                .check(body)
                .await;
            assert!(matches!(result, PredicateResult::Cacheable(_)));
        }

        #[tokio::test]
        async fn contains_pattern_spanning_chunks() {
            let body = BufferedBody::Passthrough(make_stream(&["hello wor", "ld"]));
            let result = PlainOperation::Contains(Bytes::from("world"))
                .check(body)
                .await;
            assert!(matches!(result, PredicateResult::Cacheable(_)));
        }

        #[tokio::test]
        async fn contains_not_found_reads_entire_stream() {
            let body = BufferedBody::Passthrough(make_stream(&["hello", " world"]));
            let result = PlainOperation::Contains(Bytes::from("goodbye"))
                .check(body)
                .await;
            let PredicateResult::NonCacheable(body) = result else {
                panic!("Expected NonCacheable, got Cacheable");
            };
            // Body should be Complete after full scan
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
        }

        #[tokio::test]
        async fn contains_single_byte_chunks() {
            let body = BufferedBody::Passthrough(make_stream(&["h", "e", "l", "l", "o"]));
            let result = PlainOperation::Contains(Bytes::from("llo"))
                .check(body)
                .await;
            assert!(matches!(result, PredicateResult::Cacheable(_)));
        }

        #[tokio::test]
        async fn starts_on_stream() {
            let body = BufferedBody::Passthrough(make_stream(&["hel", "lo ", "world"]));
            let result = PlainOperation::Starts(Bytes::from("hello"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
        }

        #[tokio::test]
        async fn starts_body_too_short() {
            let body = BufferedBody::Passthrough(make_stream(&["hi"]));
            let result = PlainOperation::Starts(Bytes::from("hello"))
                .check(body)
                .await;
            assert!(matches!(result, PredicateResult::NonCacheable(_)));
        }

        #[tokio::test]
        async fn ends_on_stream() {
            let body = BufferedBody::Passthrough(make_stream(&["hello ", "world"]));
            let result = PlainOperation::Ends(Bytes::from("world")).check(body).await;
            assert!(matches!(result, PredicateResult::Cacheable(_)));
        }

        #[tokio::test]
        async fn regexp_on_stream() {
            let body = BufferedBody::Passthrough(make_stream(&["status: ", "200"]));
            let regex = regex::bytes::Regex::new(r"\d+").unwrap();
            let result = PlainOperation::RegExp(regex).check(body).await;
            assert!(matches!(result, PredicateResult::Cacheable(_)));
        }
    }
}

// ===========================================================================
// Case 3: Body present, with trailers
// ===========================================================================

/// HTTP/2 responses carrying both a body and trailing headers.
/// The primary use case is gRPC, where `grpc-status` and `grpc-message`
/// are sent as trailers after the response body.
mod case3_body_with_trailers {
    use super::*;

    /// Body and trailers already buffered (e.g., cache hit with stored trailers).
    mod complete {
        use super::*;

        #[tokio::test]
        async fn poll_frame_data_then_trailers_then_none() {
            let mut body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello")),
                trailers: Some(make_trailers()),
            };

            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("hello"));

            let frame = body.frame().await.unwrap().unwrap();
            let trailers = frame.into_trailers().unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");

            assert!(body.frame().await.is_none());
        }

        #[tokio::test]
        async fn is_end_stream_false_with_pending_trailers() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: None,
                trailers: Some(make_trailers()),
            };
            assert!(!body.is_end_stream());
        }

        #[tokio::test]
        async fn collect_preserves_trailers() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello")),
                trailers: Some(make_trailers()),
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello"));
            let trailers = collected.trailers.unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");
            assert_eq!(trailers.get("grpc-message").unwrap(), "OK");
        }

        // -- Through predicates --

        #[tokio::test]
        async fn eq_preserves_trailers() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello")),
                trailers: Some(make_trailers()),
            };
            let result = PlainOperation::Eq(Bytes::from("hello")).check(body).await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello"));
            let trailers = collected.trailers.unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");
            assert_eq!(trailers.get("grpc-message").unwrap(), "OK");
        }

        #[tokio::test]
        async fn contains_preserves_trailers() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello world")),
                trailers: Some(make_trailers()),
            };
            let result = PlainOperation::Contains(Bytes::from("world"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            assert!(collected.trailers.is_some());
        }

        #[tokio::test]
        async fn starts_preserves_trailers() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello world")),
                trailers: Some(make_trailers()),
            };
            let result = PlainOperation::Starts(Bytes::from("hello"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            assert!(collected.trailers.is_some());
        }

        #[tokio::test]
        async fn ends_preserves_trailers() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("hello world")),
                trailers: Some(make_trailers()),
            };
            let result = PlainOperation::Ends(Bytes::from("world")).check(body).await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert!(collected.trailers.is_some());
        }

        #[tokio::test]
        async fn regexp_preserves_trailers() {
            let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
                data: Some(Bytes::from("status: 200")),
                trailers: Some(make_trailers()),
            };
            let regex = regex::bytes::Regex::new(r"\d+").unwrap();
            let result = PlainOperation::RegExp(regex).check(body).await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert!(collected.trailers.is_some());
        }
    }

    /// Partially consumed — prefix buffered, trailers may be in the remaining stream.
    mod partial {
        use super::*;

        #[tokio::test]
        async fn poll_frame_prefix_then_stream_data_then_trailers() {
            let stream = make_stream_with_trailers(&["remaining"], make_trailers());
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("prefix-")), Remaining::Body(stream));
            let mut body = BufferedBody::Partial(partial);

            // Prefix
            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("prefix-"));

            // Stream data
            let frame = body.frame().await.unwrap().unwrap();
            assert_eq!(frame.into_data().unwrap(), Bytes::from("remaining"));

            // Trailers
            let frame = body.frame().await.unwrap().unwrap();
            let trailers = frame.into_trailers().unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");

            assert!(body.frame().await.is_none());
        }

        #[tokio::test]
        async fn collect_preserves_trailers() {
            let stream = make_stream_with_trailers(&[" world"], make_trailers());
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("hello")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);

            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            let trailers = collected.trailers.unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");
        }

        // -- Through predicates --

        #[tokio::test]
        async fn eq_on_partial_preserves_trailers() {
            let stream = make_stream_with_trailers(&[" world"], make_trailers());
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("hello")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);

            let result = PlainOperation::Eq(Bytes::from("hello world"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            assert!(collected.trailers.is_some());
        }

        #[tokio::test]
        async fn contains_on_partial_preserves_trailers() {
            let stream = make_stream_with_trailers(&[" world"], make_trailers());
            let partial =
                PartialBufferedBody::new(Some(Bytes::from("hello")), Remaining::Body(stream));
            let body = BufferedBody::Partial(partial);

            let result = PlainOperation::Contains(Bytes::from("world"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert!(collected.trailers.is_some());
        }
    }

    /// Untouched stream: data frames followed by a trailer frame.
    mod passthrough {
        use super::*;

        #[tokio::test]
        async fn poll_frame_data_chunks_then_trailers() {
            let stream = make_stream_with_trailers(&["chunk1", "chunk2"], make_trailers());
            let mut body = BufferedBody::Passthrough(stream);

            let f1 = body.frame().await.unwrap().unwrap().into_data().unwrap();
            let f2 = body.frame().await.unwrap().unwrap().into_data().unwrap();
            assert_eq!(f1, Bytes::from("chunk1"));
            assert_eq!(f2, Bytes::from("chunk2"));

            let frame = body.frame().await.unwrap().unwrap();
            let trailers = frame.into_trailers().unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");

            assert!(body.frame().await.is_none());
        }

        #[tokio::test]
        async fn collect_preserves_trailers_from_stream() {
            let stream = make_stream_with_trailers(&["hello", " world"], make_trailers());
            let body = BufferedBody::Passthrough(stream);

            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            let trailers = collected.trailers.unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");
        }

        #[tokio::test]
        async fn eq_preserves_trailers_from_stream() {
            let stream = make_stream_with_trailers(&["hello"], make_trailers());
            let body = BufferedBody::Passthrough(stream);

            let result = PlainOperation::Eq(Bytes::from("hello")).check(body).await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello"));
            let trailers = collected.trailers.unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");
        }

        #[tokio::test]
        async fn contains_found_mid_stream_preserves_trailers() {
            // Pattern found early — streaming_search returns Partial.
            // Remaining stream still has more data + trailers.
            let stream = make_stream_with_trailers(&["hello world", " more data"], make_trailers());
            let body = BufferedBody::Passthrough(stream);

            let result = PlainOperation::Contains(Bytes::from("hello"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            // All data should be recoverable
            assert!(!collected.data.is_empty());
            // Trailers must be preserved (this was Bug #1 from PR review)
            assert!(collected.trailers.is_some());
        }

        #[tokio::test]
        async fn contains_not_found_captures_trailers() {
            // Pattern not found — streaming_search reads entire stream including trailers.
            let stream = make_stream_with_trailers(&["hello", " world"], make_trailers());
            let body = BufferedBody::Passthrough(stream);

            let result = PlainOperation::Contains(Bytes::from("goodbye"))
                .check(body)
                .await;
            let PredicateResult::NonCacheable(body) = result else {
                panic!("Expected NonCacheable, got Cacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            let trailers = collected.trailers.unwrap();
            assert_eq!(trailers.get("grpc-status").unwrap(), "0");
        }

        #[tokio::test]
        async fn starts_with_limit_under_body_preserves_stream_and_trailers() {
            // Starts reads a prefix via collect_exact, trailers remain in stream.
            let stream = make_stream_with_trailers(&["hello world"], make_trailers());
            let body = BufferedBody::Passthrough(stream);

            let result = PlainOperation::Starts(Bytes::from("hello"))
                .check(body)
                .await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert_eq!(collected.data, Bytes::from("hello world"));
            assert!(collected.trailers.is_some());
        }

        #[tokio::test]
        async fn ends_preserves_trailers_from_stream() {
            let stream = make_stream_with_trailers(&["hello ", "world"], make_trailers());
            let body = BufferedBody::Passthrough(stream);

            let result = PlainOperation::Ends(Bytes::from("world")).check(body).await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert!(collected.trailers.is_some());
        }

        #[tokio::test]
        async fn regexp_preserves_trailers_from_stream() {
            let stream = make_stream_with_trailers(&["code: 200"], make_trailers());
            let body = BufferedBody::Passthrough(stream);

            let regex = regex::bytes::Regex::new(r"\d+").unwrap();
            let result = PlainOperation::RegExp(regex).check(body).await;
            let PredicateResult::Cacheable(body) = result else {
                panic!("Expected Cacheable, got NonCacheable");
            };
            let collected = body.collect().await.unwrap();
            assert!(collected.trailers.is_some());
        }
    }
}

// ===========================================================================
// Case 4: No body, but trailers present
// ===========================================================================

/// gRPC "Trailers-Only" response — the server sends a single HEADERS frame
/// with both `:status` (in headers) and `grpc-status` (in trailers), with
/// no DATA frames. Used for immediate errors or empty-response RPCs.
///
/// `BufferedBody::Complete { data: None, trailers: Some(...) }`
mod case4_no_body_with_trailers {
    use super::*;

    #[tokio::test]
    async fn poll_frame_yields_trailers_only() {
        let mut body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: Some(make_trailers()),
        };

        let frame = body.frame().await.unwrap().unwrap();
        let trailers = frame.into_trailers().unwrap();
        assert_eq!(trailers.get("grpc-status").unwrap(), "0");

        assert!(body.frame().await.is_none());
    }

    #[tokio::test]
    async fn collect_preserves_trailers() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: Some(make_trailers()),
        };
        let collected = body.collect().await.unwrap();
        assert!(collected.data.is_empty());
        let trailers = collected.trailers.unwrap();
        assert_eq!(trailers.get("grpc-status").unwrap(), "0");
    }

    #[tokio::test]
    async fn eq_empty_matches_and_preserves_trailers() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: Some(make_trailers()),
        };
        let result = PlainOperation::Eq(Bytes::new()).check(body).await;
        let PredicateResult::Cacheable(body) = result else {
            panic!("Expected Cacheable, got NonCacheable");
        };
        let collected = body.collect().await.unwrap();
        assert!(collected.data.is_empty());
        assert!(collected.trailers.is_some());
    }

    #[tokio::test]
    async fn contains_does_not_match_but_preserves_trailers() {
        let body: BufferedBody<Full<Bytes>> = BufferedBody::Complete {
            data: None,
            trailers: Some(make_trailers()),
        };
        let result = PlainOperation::Contains(Bytes::from("hello"))
            .check(body)
            .await;
        let PredicateResult::NonCacheable(body) = result else {
            panic!("Expected NonCacheable, got Cacheable");
        };
        let collected = body.collect().await.unwrap();
        assert!(collected.trailers.is_some());
    }
}

// ===========================================================================
// CollectExactResult::into_buffered_body()
// ===========================================================================

/// Reconstruction tests — verifying that `into_buffered_body()` correctly
/// reassembles a `BufferedBody` from a `CollectExactResult`.
mod into_buffered_body {
    use super::*;

    #[tokio::test]
    async fn at_least_with_remaining_stream() {
        let stream = make_stream(&["hello ", "world"]);
        let body = BufferedBody::Passthrough(stream);

        let result = body.collect_exact(5).await;

        match result {
            CollectExactResult::AtLeast { ref buffered, .. } => {
                assert!(buffered.len() >= 5);
            }
            other => panic!("Expected AtLeast, got {:?}", other),
        }

        let reconstructed = result.into_buffered_body();
        let collected = reconstructed.collect().await.unwrap();
        assert_eq!(collected.data, Bytes::from("hello world"));
    }

    #[tokio::test]
    async fn incomplete_becomes_complete() {
        let body = BufferedBody::Passthrough(make_stream(&["hi"]));

        let result = body.collect_exact(1000).await;

        match &result {
            CollectExactResult::Incomplete { buffered, .. } => {
                assert_eq!(buffered.as_ref().unwrap(), &Bytes::from("hi"));
            }
            other => panic!("Expected Incomplete, got {:?}", other),
        }

        let reconstructed = result.into_buffered_body();
        let collected = reconstructed.collect().await.unwrap();
        assert_eq!(collected.data, Bytes::from("hi"));
    }

    #[tokio::test]
    async fn incomplete_with_trailers() {
        let stream = make_stream_with_trailers(&["data"], make_trailers());
        let body = BufferedBody::Passthrough(stream);

        let result = body.collect_exact(1000).await;
        let reconstructed = result.into_buffered_body();

        let collected = reconstructed.collect().await.unwrap();
        assert_eq!(collected.data, Bytes::from("data"));
        assert!(collected.trailers.is_some());
    }

    #[tokio::test]
    async fn at_least_preserves_remaining_trailers() {
        let stream = make_stream_with_trailers(&["hello world"], make_trailers());
        let body = BufferedBody::Passthrough(stream);

        let result = body.collect_exact(5).await;
        let reconstructed = result.into_buffered_body();

        let collected = reconstructed.collect().await.unwrap();
        assert_eq!(collected.data, Bytes::from("hello world"));
        assert!(collected.trailers.is_some());
    }

    #[tokio::test]
    async fn empty_stream_incomplete() {
        let body = BufferedBody::Passthrough(make_stream(&[]));
        let result = body.collect_exact(10).await;
        let reconstructed = result.into_buffered_body();

        let collected = reconstructed.collect().await.unwrap();
        assert!(collected.data.is_empty());
    }
}

// ===========================================================================
// PartialBufferedBody
// ===========================================================================

/// Tests for the inner `PartialBufferedBody` type:
/// `new()`, `prefix()`, and `into_parts()`.
mod partial_buffered_body {
    use super::*;

    #[test]
    fn prefix_returns_buffered_bytes() {
        let stream = make_stream(&["remaining"]);
        let partial =
            PartialBufferedBody::new(Some(Bytes::from("prefix")), Remaining::Body(stream));
        assert_eq!(partial.prefix().unwrap(), &Bytes::from("prefix"));
    }

    #[test]
    fn prefix_returns_none_when_empty() {
        let stream = make_stream(&["data"]);
        let partial = PartialBufferedBody::new(None, Remaining::Body(stream));
        assert!(partial.prefix().is_none());
    }

    #[test]
    fn into_parts_decomposes() {
        let stream = make_stream(&["remaining"]);
        let partial =
            PartialBufferedBody::new(Some(Bytes::from("prefix")), Remaining::Body(stream));

        let (prefix, remaining) = partial.into_parts();
        assert_eq!(prefix.unwrap(), Bytes::from("prefix"));
        assert!(matches!(remaining, Remaining::Body(_)));
    }

    #[test]
    fn into_parts_with_error() {
        let partial: PartialBufferedBody<IoErrorStream> = PartialBufferedBody::new(
            Some(Bytes::from("prefix")),
            Remaining::Error(Some(std::io::Error::other("test"))),
        );

        let (prefix, remaining) = partial.into_parts();
        assert_eq!(prefix.unwrap(), Bytes::from("prefix"));
        assert!(matches!(remaining, Remaining::Error(Some(_))));
    }
}
