use hitbox_backend::CacheKeyFormat;
use hitbox_core::CacheKey;

#[test]
fn test_key_format_bincode_roundtrip() {
    let key = CacheKey::from_slice(&[("method", Some("GET")), ("path", Some("/users"))]);
    let format = CacheKeyFormat::Bitcode;

    let serialized = format.serialize(&key).expect("Failed to serialize");
    let deserialized = format
        .deserialize(&serialized)
        .expect("Failed to deserialize");

    assert_eq!(key, deserialized);
}

#[test]
fn test_key_format_bincode_with_null_values() {
    let key = CacheKey::from_slice(&[("method", Some("GET")), (".metadata", None)]);
    let format = CacheKeyFormat::Bitcode;

    let serialized = format.serialize(&key).expect("Failed to serialize");
    let deserialized = format
        .deserialize(&serialized)
        .expect("Failed to deserialize");

    assert_eq!(key, deserialized);
}

#[test]
fn test_key_format_url_encoded_serialize() {
    let key = CacheKey::from_str("method", "GET");
    let format = CacheKeyFormat::UrlEncoded;

    let serialized = format.serialize(&key).expect("Failed to serialize");
    let serialized_str = String::from_utf8(serialized).expect("Failed to convert to string");

    // URL-encoded format: method=GET
    assert!(serialized_str.contains("method="));
    assert!(serialized_str.contains("GET"));
}
