use hitbox_backend::CacheKeyFormat;
use hitbox_core::CacheKey;

fn main() {
    // Create a cache key
    let key = CacheKey::from_slice(&[
        ("method", Some("GET")),
        ("path", Some("/api/users/123")),
        ("tenant", Some("acme-corp")),
    ]);

    println!("Original CacheKey: {:?}\n", key);

    // Demonstrate different serialization formats

    // 1. Bincode format (default, compact, fast for Redis/Tarantool)
    let bincode_format = CacheKeyFormat::Bincode;
    let bincode_serialized = bincode_format.serialize(&key).unwrap();
    println!("Bincode format ({} bytes):", bincode_serialized.len());
    println!("  (binary data, not human-readable)");

    // Can deserialize back
    let bincode_deserialized = bincode_format.deserialize(&bincode_serialized).unwrap();
    assert_eq!(key, bincode_deserialized);
    println!("  ✓ Roundtrip successful!");

    // 2. URL-encoded format (for HTTP-safe keys)
    let urlencoded_format = CacheKeyFormat::UrlEncoded;
    let urlencoded_serialized = urlencoded_format.serialize(&key).unwrap();
    println!(
        "\nURL-encoded format ({} bytes):\n  {}",
        urlencoded_serialized.len(),
        String::from_utf8_lossy(&urlencoded_serialized)
    );

    println!("\n=== Use Case Recommendations ===");
    println!("• Bincode:     Redis, Tarantool, high-performance backends (most compact, default)");
    println!("• URL-encoded: HTTP caches, CDN keys, query parameters (one-way serialization)");
}
