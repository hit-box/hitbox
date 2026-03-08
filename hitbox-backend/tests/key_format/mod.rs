//! Tests for cache key serialization formats.

use hitbox_backend::CacheKeyFormat;
use hitbox_core::{CacheKey, KeyPart};

#[test]
fn test_bitcode_roundtrip_simple() {
    let key = CacheKey::from_str("test", "value");
    let encoded = CacheKeyFormat::Bitcode.serialize(&key).unwrap();
    let decoded = CacheKeyFormat::Bitcode.deserialize(&encoded).unwrap();
    assert_eq!(format!("{}", key), format!("{}", decoded));
}

#[test]
fn test_bitcode_roundtrip_with_prefix_and_version() {
    let key = CacheKey::new("api", 1, vec![KeyPart::new("id", Some("42"))]);
    let encoded = CacheKeyFormat::Bitcode.serialize(&key).unwrap();
    let decoded = CacheKeyFormat::Bitcode.deserialize(&encoded).unwrap();

    assert_eq!(decoded.prefix(), "api");
    assert_eq!(decoded.version(), 1);
    assert_eq!(format!("{}", key), format!("{}", decoded));
}

#[test]
fn test_bitcode_roundtrip_multiple_parts() {
    let key = CacheKey::new(
        "cache",
        2,
        vec![
            KeyPart::new("method", Some("GET")),
            KeyPart::new("path", Some("/users")),
            KeyPart::new("flag", None::<&str>),
        ],
    );
    let encoded = CacheKeyFormat::Bitcode.serialize(&key).unwrap();
    let decoded = CacheKeyFormat::Bitcode.deserialize(&encoded).unwrap();

    assert_eq!(decoded.prefix(), "cache");
    assert_eq!(decoded.version(), 2);

    let parts: Vec<_> = decoded.parts().collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].key(), "method");
    assert_eq!(parts[0].value(), Some("GET"));
    assert_eq!(parts[1].key(), "path");
    assert_eq!(parts[1].value(), Some("/users"));
    assert_eq!(parts[2].key(), "flag");
    assert_eq!(parts[2].value(), None);
}

#[test]
fn test_bitcode_roundtrip_empty_parts() {
    let key = CacheKey::new("prefix", 0, vec![]);
    let encoded = CacheKeyFormat::Bitcode.serialize(&key).unwrap();
    let decoded = CacheKeyFormat::Bitcode.deserialize(&encoded).unwrap();

    assert_eq!(decoded.prefix(), "prefix");
    assert_eq!(decoded.version(), 0);
    assert_eq!(decoded.parts().count(), 0);
}
