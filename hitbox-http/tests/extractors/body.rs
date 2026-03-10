use bytes::Bytes;
use hitbox::Extractor;
use hitbox_http::extractors::body::{
    BodyConfig, BodyExtraction, BodyExtractor, IntoBodyExtraction, JqExtraction, RegexExtraction,
    Transforms,
};
use hitbox_http::extractors::transform::Transform;
use hitbox_http::extractors::{MethodConfig, NeutralExtractor, method::MethodExtractor};
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::Request;
use http_body_util::Full;
use regex::Regex;

// --- BodyConfig builder tests ---

#[test]
fn test_body_config_hash() {
    let config = BodyConfig::new().hash();
    assert!(matches!(config.into_extraction(), BodyExtraction::Hash));

    // key is accepted but ignored for hash mode
    let config = BodyConfig::new().key("custom-key").hash();
    assert!(matches!(config.into_extraction(), BodyExtraction::Hash));
}

#[test]
fn test_body_config_jq() {
    let config = BodyConfig::new().jq(".field").unwrap();
    assert!(matches!(config.into_extraction(), BodyExtraction::Jq(_)));

    assert!(BodyConfig::new().jq("invalid jq [[[").is_err());
}

#[test]
fn test_body_config_regex() {
    // basic
    let config = BodyConfig::new().regex(r"(\w+)").unwrap();
    assert!(matches!(config.into_extraction(), BodyExtraction::Regex(_)));

    // invalid
    assert!(BodyConfig::new().regex(r"(invalid[").is_err());

    // global flag
    let config = BodyConfig::new().regex(r"(\w+)").unwrap().global();
    match config.into_extraction() {
        BodyExtraction::Regex(r) => assert!(r.global),
        _ => panic!("Expected Regex extraction"),
    }

    // with key
    let config = BodyConfig::new().key("token").regex(r"(\w+)").unwrap();
    match config.into_extraction() {
        BodyExtraction::Regex(r) => assert_eq!(r.key, Some("token".to_string())),
        _ => panic!("Expected Regex extraction"),
    }

    // with transforms (builder)
    let config = BodyConfig::new()
        .transforms(Transforms::builder().full(Transform::Hash))
        .regex(r"(\w+)")
        .unwrap();
    match config.into_extraction() {
        BodyExtraction::Regex(r) => assert!(matches!(r.transforms, Transforms::FullBody(_))),
        _ => panic!("Expected Regex extraction"),
    }

    // with per-key transforms (builder)
    let config = BodyConfig::new()
        .transforms(
            Transforms::builder()
                .key("user", Transform::Lowercase)
                .key("token", Transform::Hash),
        )
        .regex(r"(?P<user>\w+):(?P<token>\w+)")
        .unwrap();
    match config.into_extraction() {
        BodyExtraction::Regex(r) => assert!(matches!(r.transforms, Transforms::PerKey(_))),
        _ => panic!("Expected Regex extraction"),
    }
}

#[test]
fn test_body_config_default() {
    let _config: BodyConfig<_> = Default::default();
}

#[test]
fn test_jq_extraction_debug() {
    let jq = JqExtraction::compile(".field").unwrap();
    let debug = format!("{:?}", jq);
    assert!(debug.contains("JqExtraction"));
}

// --- Body extractor with hash mode ---

#[tokio::test]
async fn test_body_extractor_hash_mode() {
    let json_body = r#"{"user":"alice"}"#;
    let body = Full::new(Bytes::from(json_body));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new()
        .method(MethodConfig::new())
        .body(BodyConfig::new().hash());
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let body_part = key_parts.iter().find(|p| p.key() == "body").unwrap();
    assert_eq!(
        body_part.value(),
        Some("a5cd97f8496e61268797de605913bd8a29ac3af68ec6af1bea67fdb50c2c0ebf")
    );
}

// --- Body extractor with jq mode ---

#[tokio::test]
async fn test_body_extractor_jq_mode() {
    let json_body = r#"{"user":"alice","age":30}"#;
    let body = Full::new(Bytes::from(json_body));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().body(BodyConfig::new().jq(".user").unwrap());
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let body_part = key_parts.iter().find(|p| p.key() == "body").unwrap();
    assert_eq!(body_part.value(), Some("alice"));
}

#[tokio::test]
async fn test_body_extractor_jq_mode_object_result() {
    let json_body = r#"{"user":{"name":"alice","role":"admin"}}"#;
    let body = Full::new(Bytes::from(json_body));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().body(BodyConfig::new().jq(".user").unwrap());
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    assert!(key_parts.iter().any(|p| p.key() == "name"));
    assert!(key_parts.iter().any(|p| p.key() == "role"));
}

#[tokio::test]
async fn test_body_extractor_jq_mode_edge_cases() {
    // Invalid JSON falls back to hash
    let body = Full::new(Bytes::from("not-json"));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().body(BodyConfig::new().jq(".field").unwrap());
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let body_part = key_parts.iter().find(|p| p.key() == "body").unwrap();
    assert!(body_part.value().is_some());

    // Null result
    let body = Full::new(Bytes::from(r#"{"field":null}"#));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().body(BodyConfig::new().jq(".field").unwrap());
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let body_part = key_parts.iter().find(|p| p.key() == "body").unwrap();
    assert!(body_part.value().is_none());

    // Bool result
    let body = Full::new(Bytes::from(r#"{"active":true}"#));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().body(BodyConfig::new().jq(".active").unwrap());
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let body_part = key_parts.iter().find(|p| p.key() == "body").unwrap();
    assert_eq!(body_part.value(), Some("true"));
}

// --- Body extractor jq hash for various types ---

#[tokio::test]
async fn test_body_extractor_jq_hash_types() {
    for json in [
        r#"{"val":3.14}"#,
        r#"{"val":true}"#,
        r#"{"val":null}"#,
        r#"{"val":42}"#,
    ] {
        let body = Full::new(Bytes::from(json));
        let request = Request::builder()
            .body(BufferedBody::Passthrough(body))
            .unwrap();
        let request = CacheableHttpRequest::from_request(request);
        let extractor = NeutralExtractor::new().body(BodyConfig::new().jq(".val | hash").unwrap());
        let parts = extractor.get(request, &mut Default::default()).await;
        let (_subject, cache_key) = parts.into_cache_key();
        let key_parts: Vec<_> = cache_key.parts().collect();
        let body_part = key_parts
            .iter()
            .find(|p| p.key() == "body")
            .unwrap_or_else(|| panic!("missing body part for {json}"));
        assert!(body_part.value().is_some(), "expected Some for {json}");
    }
}

// --- Body extractor with regex mode ---

#[tokio::test]
async fn test_body_extractor_regex_mode_single_match() {
    let body_text = "order_id: ABC123, status: shipped";
    let body = Full::new(Bytes::from(body_text));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().body(
        BodyConfig::new()
            .key("order")
            .regex(r"order_id: (\w+)")
            .unwrap(),
    );
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let order_part = key_parts.iter().find(|p| p.key() == "order").unwrap();
    assert_eq!(order_part.value(), Some("ABC123"));
}

#[tokio::test]
async fn test_body_extractor_regex_mode_global() {
    let body_text = "id=100 id=200 id=300";
    let body = Full::new(Bytes::from(body_text));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extractor = NeutralExtractor::new().body(
        BodyConfig::new()
            .key("id")
            .regex(r"id=(\d+)")
            .unwrap()
            .global(),
    );
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    let id_parts: Vec<_> = key_parts.iter().filter(|p| p.key() == "id").collect();
    assert_eq!(id_parts.len(), 3);
}

#[tokio::test]
async fn test_body_extractor_regex_mode_named_groups() {
    let body_text = "user: alice, role: admin";
    let body = Full::new(Bytes::from(body_text));
    let request = Request::builder()
        .body(BufferedBody::Passthrough(body))
        .unwrap();
    let request = CacheableHttpRequest::from_request(request);
    let extraction = BodyExtraction::Regex(RegexExtraction {
        regex: Regex::new(r"user: (?P<user>\w+), role: (?P<role>\w+)").unwrap(),
        key: None,
        global: false,
        transforms: Transforms::None,
    });
    let extractor = NeutralExtractor::new().body(extraction);
    let parts = extractor.get(request, &mut Default::default()).await;
    let (_subject, cache_key) = parts.into_cache_key();
    let key_parts: Vec<_> = cache_key.parts().collect();
    assert!(key_parts.iter().any(|p| p.key() == "user"));
    assert!(key_parts.iter().any(|p| p.key() == "role"));
}
