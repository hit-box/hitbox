use hitbox_backend::CacheKeyFormat;
use hitbox_core::CacheKey;

fn main() {
    // Create a cache key
    let key = CacheKey::from_slice(&[
        ("method", "GET"),
        ("path", "/api/users/123"),
        ("tenant", "acme-corp"),
    ]);

    println!("Original CacheKey: {:?}\n", key);

    // Demonstrate different serialization formats

    // 1. String format (default, human-readable)
    let string_format = CacheKeyFormat::String;
    let string_serialized = string_format.serialize(&key).unwrap();
    println!(
        "String format ({} bytes):\n  {}",
        string_serialized.len(),
        String::from_utf8_lossy(&string_serialized)
    );

    // 2. JSON format (debugging, cross-platform)
    let json_format = CacheKeyFormat::Json;
    let json_serialized = json_format.serialize(&key).unwrap();
    println!(
        "\nJSON format ({} bytes):\n  {}",
        json_serialized.len(),
        String::from_utf8_lossy(&json_serialized)
    );

    // Can deserialize back
    let json_deserialized = json_format.deserialize(&json_serialized).unwrap();
    assert_eq!(key, json_deserialized);
    println!("  ✓ Roundtrip successful!");

    // 3. Bincode format (compact, fast for Redis/Tarantool)
    let bincode_format = CacheKeyFormat::Bincode;
    let bincode_serialized = bincode_format.serialize(&key).unwrap();
    println!("\nBincode format ({} bytes):", bincode_serialized.len());
    println!("  (binary data, not human-readable)");

    // Can deserialize back
    let bincode_deserialized = bincode_format.deserialize(&bincode_serialized).unwrap();
    assert_eq!(key, bincode_deserialized);
    println!("  ✓ Roundtrip successful!");

    // 4. URL-encoded format (for HTTP-safe keys)
    let urlencoded_format = CacheKeyFormat::UrlEncoded;
    let urlencoded_serialized = urlencoded_format.serialize(&key).unwrap();
    println!(
        "\nURL-encoded format ({} bytes):\n  {}",
        urlencoded_serialized.len(),
        String::from_utf8_lossy(&urlencoded_serialized)
    );

    // Size comparison
    println!("\n=== Size Comparison ===");
    println!("String:      {} bytes", string_serialized.len());
    println!("JSON:        {} bytes", json_serialized.len());
    println!("Bincode:     {} bytes ({}% of JSON)",
        bincode_serialized.len(),
        (bincode_serialized.len() * 100) / json_serialized.len()
    );
    println!("URL-encoded: {} bytes", urlencoded_serialized.len());

    println!("\n=== Use Case Recommendations ===");
    println!("• String:      Logging, debugging, simple file-based caches");
    println!("• JSON:        Cross-language compatibility, debugging, configuration");
    println!("• Bincode:     Redis, Tarantool, high-performance backends (most compact)");
    println!("• URL-encoded: HTTP caches, CDN keys, query parameters");
}
