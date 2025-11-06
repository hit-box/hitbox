use hitbox_backend::serializer::Format;
/// Complete example showing how to configure both key and value serialization formats
/// for optimal Redis backend performance.
use hitbox_backend::{Backend, CacheKeyFormat};
use hitbox_redis::RedisBackend;

fn main() {
    println!("=== Complete Backend Configuration Guide ===\n");

    // Scenario 1: Development/Debugging Configuration
    // - URL-encoded keys for readability
    // - JSON values for easy inspection
    println!("📝 Scenario 1: Development/Debugging");
    let dev_backend = RedisBackend::builder()
        .server("redis://127.0.0.1:6379/0".to_string())
        .key_format(CacheKeyFormat::UrlEncoded) // Human-readable keys
        .value_format(Format::Json) // Human-readable values
        .build()
        .expect("Failed to create dev backend");

    println!("   Key format:   {:?}", dev_backend.key_format());
    println!("   Value format: {:?}", dev_backend.value_format());
    println!("   Benefit: Easy debugging with redis-cli");
    println!("   Trade-off: Slightly larger keys than Bitcode\n");

    // Scenario 2: Production High-Performance Configuration
    // - Compact binary keys and values
    // - Minimal memory footprint
    // - Fastest serialization
    println!("🚀 Scenario 2: Production (High Performance)");
    let prod_backend = RedisBackend::builder()
        .server("redis://127.0.0.1:6379/1".to_string())
        .key_format(CacheKeyFormat::Bitcode) // Compact binary keys
        .value_format(Format::Json) // Standard JSON values (or Bincode for max performance)
        .build()
        .expect("Failed to create prod backend");

    println!("   Key format:   {:?}", prod_backend.key_format());
    println!("   Value format: {:?}", prod_backend.value_format());
    println!("   Benefit: 20-30% less memory, faster serialization");
    println!("   Trade-off: Keys not human-readable\n");

    // Scenario 3: CDN/HTTP Caching Integration
    // - URL-safe keys for HTTP headers
    // - JSON values for cross-platform compatibility
    println!("🌐 Scenario 3: CDN/HTTP Integration");
    let cdn_backend = RedisBackend::builder()
        .server("redis://127.0.0.1:6379/2".to_string())
        .key_format(CacheKeyFormat::UrlEncoded) // HTTP-safe keys
        .value_format(Format::Json) // Cross-platform values
        .build()
        .expect("Failed to create CDN backend");

    println!("   Key format:   {:?}", cdn_backend.key_format());
    println!("   Value format: {:?}", cdn_backend.value_format());
    println!("   Benefit: Keys safe for Vary headers, CDN integration");
    println!("   Use case: Edge caching, HTTP cache control\n");

    // Scenario 4: Maximum Efficiency Configuration
    // - Bitcode keys for minimal memory
    // - Bencode values for compact storage
    println!("⚡ Scenario 4: Maximum Efficiency (Bitcode keys + Bencode values)");
    let efficient_backend = RedisBackend::builder()
        .server("redis://127.0.0.1:6379/3".to_string())
        .key_format(CacheKeyFormat::Bitcode) // Most compact keys
        .value_format(Format::Bencode) // Compact values
        .build()
        .expect("Failed to create efficient backend");

    println!("   Key format:   {:?}", efficient_backend.key_format());
    println!("   Value format: {:?}", efficient_backend.value_format());
    println!("   Benefit: Minimal memory footprint, fast serialization");
    println!("   Use case: High-scale production, limited memory\n");

    println!("=== Format Comparison Table ===");
    println!("┌─────────────┬────────────┬──────────────┬───────────────────┐");
    println!("│ Format      │ Key Size   │ Readable     │ Best For          │");
    println!("├─────────────┼────────────┼──────────────┼───────────────────┤");
    println!("│ Bitcode     │ Smallest   │ ✗ No         │ Production, Speed │");
    println!("│ UrlEncoded  │ Medium     │ ✓ Yes        │ HTTP, CDN, Debug  │");
    println!("└─────────────┴────────────┴──────────────┴───────────────────┘\n");

    println!("=== Recommendations ===");
    println!("• Development:   UrlEncoded keys + JSON values");
    println!("• Production:    Bitcode keys + Bencode values (maximum efficiency)");
    println!("• CDN/Edge:      UrlEncoded keys + JSON values");
    println!("• High-scale:    Bitcode keys + Bencode values");
}
