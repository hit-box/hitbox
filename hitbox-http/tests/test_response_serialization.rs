use bytes::Bytes;
use hitbox::CacheableResponse;
use hitbox_http::{BufferedBody, CacheableHttpResponse};
use http::{HeaderValue, Response, StatusCode};
use http_body_util::Full;

type TestBody = BufferedBody<Full<Bytes>>;

async fn body_to_string(body: TestBody) -> String {
    let bytes = body.collect().await.unwrap();
    String::from_utf8_lossy(&bytes).to_string()
}

fn compare_responses(original: &Response<TestBody>, restored: &Response<TestBody>) -> Vec<String> {
    let mut differences = Vec::new();

    if original.status() != restored.status() {
        differences.push(format!(
            "Status code mismatch: expected {}, got {}",
            original.status(),
            restored.status()
        ));
    }

    for (name, value) in original.headers() {
        let restored_values: Vec<_> = restored.headers().get_all(name).iter().collect();
        if !restored_values.contains(&value) {
            differences.push(format!(
                "Header '{}' value mismatch: original has '{}', but not found in restored response",
                name,
                value.to_str().unwrap_or("<binary>")
            ));
        }
    }

    for (name, value) in restored.headers() {
        let original_values: Vec<_> = original.headers().get_all(name).iter().collect();
        if !original_values.contains(&value) {
            differences.push(format!(
                "Extra header in restored response: '{}' = '{}'",
                name,
                value.to_str().unwrap_or("<binary>")
            ));
        }
    }

    for name in original.headers().keys() {
        let original_count = original.headers().get_all(name).iter().count();
        let restored_count = restored.headers().get_all(name).iter().count();
        if original_count != restored_count {
            differences.push(format!(
                "Header '{}' count mismatch: original has {} values, restored has {} values",
                name, original_count, restored_count
            ));
        }
    }

    differences
}

async fn assert_responses_equal(original: Response<TestBody>, restored: Response<TestBody>) {
    let (original_parts, original_body) = original.into_parts();
    let (restored_parts, restored_body) = restored.into_parts();

    let original_bytes = original_body.collect().await.unwrap();
    let restored_bytes = restored_body.collect().await.unwrap();

    if original_bytes != restored_bytes {
        panic!(
            "Response body mismatch:\nExpected: {:?}\nGot: {:?}",
            String::from_utf8_lossy(&original_bytes),
            String::from_utf8_lossy(&restored_bytes)
        );
    }

    let original = Response::from_parts(
        original_parts,
        BufferedBody::Complete(Some(original_bytes.clone())),
    );
    let restored =
        Response::from_parts(restored_parts, BufferedBody::Complete(Some(restored_bytes)));

    let differences = compare_responses(&original, &restored);

    if !differences.is_empty() {
        panic!(
            "Response comparison failed with {} differences:\n{}",
            differences.len(),
            differences.join("\n")
        );
    }
}

/// Full serialization roundtrip: Response -> Cacheable -> Serializable -> bytes -> Serializable -> Cacheable -> Response
async fn roundtrip_test(response: Response<TestBody>) -> Response<TestBody> {
    let cacheable = CacheableHttpResponse::from_response(response);

    let cache_policy = cacheable.into_cached().await;
    let serializable = match cache_policy {
        hitbox::CachePolicy::Cacheable(s) => s,
        hitbox::CachePolicy::NonCacheable(_) => panic!("Expected cacheable"),
    };

    let serialized = bincode::serde::encode_to_vec(&serializable, bincode::config::standard())
        .expect("Failed to serialize");

    let (deserialized, _len): (_, _) =
        bincode::serde::decode_from_slice(&serialized, bincode::config::standard())
            .expect("Failed to deserialize");

    let cacheable_restored = CacheableHttpResponse::<Full<Bytes>>::from_cached(deserialized).await;

    cacheable_restored.into_response()
}

#[tokio::test]
async fn test_basic_response() {
    let original = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("{}"))))
        .unwrap();

    let original_clone = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("{}"))))
        .unwrap();

    let restored = roundtrip_test(original).await;

    assert_responses_equal(original_clone, restored).await;
}

#[tokio::test]
async fn test_multiple_header_values() {
    let mut original = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(
            r#"{"status":"ok"}"#,
        ))))
        .unwrap();

    let headers = original.headers_mut();
    headers.append(
        "set-cookie",
        HeaderValue::from_static("session=abc123; Path=/"),
    );
    headers.append(
        "set-cookie",
        HeaderValue::from_static("token=xyz789; Secure"),
    );
    headers.append("set-cookie", HeaderValue::from_static("user_id=42"));

    let mut original_clone = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(
            r#"{"status":"ok"}"#,
        ))))
        .unwrap();
    let headers_clone = original_clone.headers_mut();
    headers_clone.append(
        "set-cookie",
        HeaderValue::from_static("session=abc123; Path=/"),
    );
    headers_clone.append(
        "set-cookie",
        HeaderValue::from_static("token=xyz789; Secure"),
    );
    headers_clone.append("set-cookie", HeaderValue::from_static("user_id=42"));

    let restored = roundtrip_test(original).await;

    assert_responses_equal(original_clone, restored).await;
}

#[tokio::test]
async fn test_special_header_values() {
    let original = Response::builder()
        .status(200)
        .header("x-empty", "")
        .header("x-whitespace", "  value  ")
        .header("x-special", "value-with-dash_and_underscore")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("test"))))
        .unwrap();

    let restored = roundtrip_test(original).await;

    assert_eq!(restored.headers().get("x-empty").unwrap(), "");
    assert_eq!(restored.headers().get("x-whitespace").unwrap(), "  value  ");
    assert_eq!(
        restored.headers().get("x-special").unwrap(),
        "value-with-dash_and_underscore"
    );
}

#[tokio::test]
async fn test_different_status_codes() {
    let test_cases = vec![
        (200, "OK"),
        (201, "Created"),
        (204, "No Content"),
        (301, "Moved Permanently"),
        (302, "Found"),
        (304, "Not Modified"),
        (400, "Bad Request"),
        (404, "Not Found"),
        (500, "Internal Server Error"),
        (503, "Service Unavailable"),
    ];

    for (code, body) in test_cases {
        let original = Response::builder()
            .status(code)
            .header("content-type", "text/plain")
            .body(BufferedBody::Passthrough(Full::new(Bytes::from(body))))
            .unwrap();

        let restored = roundtrip_test(original).await;

        assert_eq!(
            restored.status().as_u16(),
            code,
            "Status code mismatch for {}",
            code
        );
        let restored_body = body_to_string(restored.into_body()).await;
        assert_eq!(restored_body, body, "Body mismatch for status {}", code);
    }
}

#[tokio::test]
async fn test_different_body_types() {
    let original = Response::builder()
        .status(204)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(""))))
        .unwrap();
    let restored = roundtrip_test(original).await;
    assert_eq!(body_to_string(restored.into_body()).await, "");

    let original = Response::builder()
        .status(200)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("small"))))
        .unwrap();
    let restored = roundtrip_test(original).await;
    assert_eq!(body_to_string(restored.into_body()).await, "small");

    let large_body = "x".repeat(10000);
    let large_body_bytes = Bytes::from(large_body.clone());
    let original = Response::builder()
        .status(200)
        .body(BufferedBody::Passthrough(Full::new(large_body_bytes)))
        .unwrap();
    let restored = roundtrip_test(original).await;
    assert_eq!(body_to_string(restored.into_body()).await, large_body);
}

#[tokio::test]
async fn test_binary_data() {
    let binary_data = vec![0u8, 1, 2, 255, 254, 128, 127];
    let original = Response::builder()
        .status(200)
        .header("content-type", "application/octet-stream")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(
            binary_data,
        ))))
        .unwrap();

    let restored = roundtrip_test(original).await;

    assert_eq!(restored.status(), StatusCode::OK);
    assert_eq!(
        restored.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
}

#[tokio::test]
async fn test_no_extra_headers() {
    let original = Response::builder()
        .status(200)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("minimal"))))
        .unwrap();

    let restored = roundtrip_test(original).await;

    assert_eq!(restored.status(), StatusCode::OK);
    let body = body_to_string(restored.into_body()).await;
    assert_eq!(body, "minimal");
}

#[tokio::test]
async fn test_many_headers() {
    let mut builder = Response::builder().status(200);

    for i in 0..50 {
        builder = builder.header(format!("x-custom-{}", i), format!("value-{}", i));
    }

    let original = builder
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(
            "many headers",
        ))))
        .unwrap();
    let original_header_count = original.headers().len();

    let restored = roundtrip_test(original).await;

    assert_eq!(restored.headers().len(), original_header_count);
    assert_eq!(restored.headers().get("x-custom-0").unwrap(), "value-0");
    assert_eq!(restored.headers().get("x-custom-25").unwrap(), "value-25");
    assert_eq!(restored.headers().get("x-custom-49").unwrap(), "value-49");

    let body = body_to_string(restored.into_body()).await;
    assert_eq!(body, "many headers");
}

#[tokio::test]
async fn test_long_header_values() {
    let long_value = "x".repeat(1000);

    let original = Response::builder()
        .status(200)
        .header("x-long-header", long_value.as_str())
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("test"))))
        .unwrap();

    let restored = roundtrip_test(original).await;

    assert_eq!(
        restored.headers().get("x-long-header").unwrap(),
        long_value.as_str()
    );
}

#[tokio::test]
async fn test_common_headers() {
    let original = Response::builder()
        .status(200)
        .header("content-type", "application/json; charset=utf-8")
        .header("cache-control", "max-age=3600, public")
        .header("etag", "\"686897696a7c876b7e\"")
        .header("last-modified", "Wed, 21 Oct 2015 07:28:00 GMT")
        .header("vary", "Accept-Encoding")
        .header("server", "hitbox/1.0")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(
            r#"{"data":"test"}"#,
        ))))
        .unwrap();

    let restored = roundtrip_test(original).await;

    assert_eq!(
        restored.headers().get("content-type").unwrap(),
        "application/json; charset=utf-8"
    );
    assert_eq!(
        restored.headers().get("cache-control").unwrap(),
        "max-age=3600, public"
    );
    assert_eq!(
        restored.headers().get("etag").unwrap(),
        "\"686897696a7c876b7e\""
    );
    assert_eq!(
        restored.headers().get("last-modified").unwrap(),
        "Wed, 21 Oct 2015 07:28:00 GMT"
    );
    assert_eq!(restored.headers().get("vary").unwrap(), "Accept-Encoding");
    assert_eq!(restored.headers().get("server").unwrap(), "hitbox/1.0");

    let body = body_to_string(restored.into_body()).await;
    assert_eq!(body, r#"{"data":"test"}"#);
}

#[tokio::test]
async fn test_redirect_response() {
    let original = Response::builder()
        .status(302)
        .header("location", "https://example.com/new-location")
        .header("cache-control", "no-cache")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(""))))
        .unwrap();

    let restored = roundtrip_test(original).await;

    assert_eq!(restored.status(), StatusCode::FOUND);
    assert_eq!(
        restored.headers().get("location").unwrap(),
        "https://example.com/new-location"
    );
    assert_eq!(restored.headers().get("cache-control").unwrap(), "no-cache");
}

#[tokio::test]
async fn test_json_response_with_unicode() {
    let json_body = r#"{"message":"Hello 世界 🌍","emoji":"🚀"}"#;

    let original = Response::builder()
        .status(200)
        .header("content-type", "application/json; charset=utf-8")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(json_body))))
        .unwrap();

    let restored = roundtrip_test(original).await;

    assert_eq!(restored.status(), StatusCode::OK);
    let body = body_to_string(restored.into_body()).await;
    assert_eq!(body, json_body);
}

#[tokio::test]
#[should_panic(expected = "Response comparison failed with")]
async fn test_comparison_detects_status_mismatch() {
    let original = Response::builder()
        .status(200)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("test"))))
        .unwrap();

    let different = Response::builder()
        .status(404)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("test"))))
        .unwrap();

    assert_responses_equal(original, different).await;
}

#[tokio::test]
#[should_panic(expected = "Response body mismatch")]
async fn test_comparison_detects_body_mismatch() {
    let original = Response::builder()
        .status(200)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(
            "original body",
        ))))
        .unwrap();

    let different = Response::builder()
        .status(200)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from(
            "different body",
        ))))
        .unwrap();

    assert_responses_equal(original, different).await;
}

#[tokio::test]
#[should_panic(expected = "Response comparison failed with")]
async fn test_comparison_detects_header_mismatch() {
    let original = Response::builder()
        .status(200)
        .header("x-custom", "value1")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("test"))))
        .unwrap();

    let different = Response::builder()
        .status(200)
        .header("x-custom", "value2")
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("test"))))
        .unwrap();

    assert_responses_equal(original, different).await;
}

#[tokio::test]
#[should_panic(expected = "count mismatch")]
async fn test_comparison_detects_multivalue_count_mismatch() {
    let mut original = Response::builder()
        .status(200)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("test"))))
        .unwrap();
    original
        .headers_mut()
        .append("set-cookie", HeaderValue::from_static("a=1"));
    original
        .headers_mut()
        .append("set-cookie", HeaderValue::from_static("b=2"));

    let mut different = Response::builder()
        .status(200)
        .body(BufferedBody::Passthrough(Full::new(Bytes::from("test"))))
        .unwrap();
    different
        .headers_mut()
        .append("set-cookie", HeaderValue::from_static("a=1"));

    assert_responses_equal(original, different).await;
}
