use hitbox_backend::CacheKeyFormat;
use hitbox_core::CacheKey;

#[test]
fn test_key_format_json_roundtrip() {
    let key = CacheKey::from_str("method", "GET");
    let format = CacheKeyFormat::Json;

    let serialized = format.serialize(&key).expect("Failed to serialize");
    println!("JSON format: {}", String::from_utf8_lossy(&serialized));
    let deserialized = format.deserialize(&serialized).expect("Failed to deserialize");

    assert_eq!(key, deserialized);
}

#[test]
fn test_key_format_bincode_roundtrip() {
    let key = CacheKey::from_slice(&[("method", Some("GET")), ("path", Some("/users"))]);
    let format = CacheKeyFormat::Bincode;

    let serialized = format.serialize(&key).expect("Failed to serialize");
    let deserialized = format.deserialize(&serialized).expect("Failed to deserialize");

    assert_eq!(key, deserialized);
}

#[test]
fn test_key_format_string_serialize() {
    let key = CacheKey::from_str("method", "GET");
    let format = CacheKeyFormat::String;

    let serialized = format.serialize(&key).expect("Failed to serialize");
    let serialized_str = String::from_utf8(serialized).expect("Failed to convert to string");

    // Should use the existing serialize() method format
    assert!(serialized_str.contains("method:GET"));
}

#[test]
fn test_key_format_json_vs_bincode_size() {
    let key = CacheKey::from_slice(&[
        ("method", Some("GET")),
        ("path", Some("/api/v1/users")),
        ("tenant", Some("acme-corp")),
    ]);

    let json_serialized = CacheKeyFormat::Json.serialize(&key).expect("Failed to serialize as JSON");
    let bincode_serialized = CacheKeyFormat::Bincode.serialize(&key).expect("Failed to serialize as bincode");

    println!("JSON size: {} bytes", json_serialized.len());
    println!("Bincode size: {} bytes", bincode_serialized.len());

    // Bincode should be more compact than JSON
    assert!(bincode_serialized.len() < json_serialized.len());
}

#[test]
fn test_key_format_with_null_values() {
    let key = CacheKey::from_slice(&[("method", Some("GET")), (".metadata", None)]);
    let format = CacheKeyFormat::Json;

    let serialized = format.serialize(&key).expect("Failed to serialize");
    println!("JSON with null: {}", String::from_utf8_lossy(&serialized));
    let deserialized = format.deserialize(&serialized).expect("Failed to deserialize");

    assert_eq!(key, deserialized);
}
