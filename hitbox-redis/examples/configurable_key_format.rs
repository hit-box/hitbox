use hitbox::CacheKey;
use hitbox_backend::{Backend, CacheKeyFormat};
use hitbox_redis::RedisBackend;

fn main() {
    println!("=== Configurable Key Format Example ===\n");

    // Example 1: Default configuration (String format)
    let backend_default = RedisBackend::builder()
        .server("redis://127.0.0.1/".to_string())
        .build()
        .expect("Failed to create default backend");

    println!("1. Default Backend:");
    println!("   Key format: {:?}", backend_default.key_format());
    println!("   Format: JSON (default)\n");

    // Example 2: Compact binary keys for high-performance use cases
    let backend_bincode = RedisBackend::builder()
        .server("redis://127.0.0.1/".to_string())
        .key_format(CacheKeyFormat::Bitcode)
        .build()
        .expect("Failed to create bincode backend");

    println!("2. Bincode Backend (compact binary keys):");
    println!("   Key format: {:?}", backend_bincode.key_format());
    println!("   Use case: High-performance, storage-efficient Redis cache");
    println!("   Benefit: Smaller keys = less memory, faster lookups\n");

    // Example 3: URL-encoded keys for HTTP-safe caching
    let backend_urlencoded = RedisBackend::builder()
        .server("redis://127.0.0.1/".to_string())
        .key_format(CacheKeyFormat::UrlEncoded)
        .build()
        .expect("Failed to create urlencoded backend");

    println!("3. URL-Encoded Backend (HTTP-safe keys):");
    println!("   Key format: {:?}", backend_urlencoded.key_format());
    println!("   Use case: CDN integration, HTTP caching headers");
    println!("   Benefit: Keys safe for use in URLs and headers\n");

    // Demonstrate key serialization with different formats
    let key = CacheKey::from_slice(&[
        ("method", Some("GET")),
        ("path", Some("/api/users/123")),
        ("tenant", Some("acme-corp")),
    ]);

    println!("=== Same CacheKey, Different Serializations ===");
    println!("CacheKey: {:?}\n", key);

    let bitcode_key = CacheKeyFormat::Bitcode.serialize(&key).unwrap();
    let urlencoded_key = CacheKeyFormat::UrlEncoded.serialize(&key).unwrap();

    println!("Bitcode format ({} bytes):", bitcode_key.len());
    println!("  [binary data - most compact]\n");

    println!("URL-encoded format ({} bytes):", urlencoded_key.len());
    println!("  {}\n", String::from_utf8_lossy(&urlencoded_key));

    println!("=== Recommendations ===");
    println!("• Use Bitcode:     For production Redis (best performance, most compact)");
    println!("• Use UrlEncoded:  For CDN/HTTP caching integration, debugging");
}
