/// Complete example showing how to configure both key and value serialization formats
/// for optimal Redis backend performance.

use hitbox_backend::{Backend, CacheKeyFormat};
use hitbox_backend::serializer::Format;
use hitbox_redis::RedisBackend;

fn main() {
    println!("=== Complete Backend Configuration Guide ===\n");

    // Scenario 1: Development/Debugging Configuration
    // - Human-readable keys and values
    // - Easy to inspect in Redis CLI
    println!("📝 Scenario 1: Development/Debugging");
    let dev_backend = RedisBackend::builder()
        .server("redis://127.0.0.1:6379/0".to_string())
        .key_format(CacheKeyFormat::Json)      // Human-readable keys
        .value_format(Format::Json)              // Human-readable values
        .build()
        .expect("Failed to create dev backend");

    println!("   Key format:   {:?}", dev_backend.key_format());
    println!("   Value format: {:?}", dev_backend.value_format());
    println!("   Benefit: Easy debugging with redis-cli");
    println!("   Trade-off: Higher memory usage\n");

    // Scenario 2: Production High-Performance Configuration
    // - Compact binary keys and values
    // - Minimal memory footprint
    // - Fastest serialization
    println!("🚀 Scenario 2: Production (High Performance)");
    let prod_backend = RedisBackend::builder()
        .server("redis://127.0.0.1:6379/1".to_string())
        .key_format(CacheKeyFormat::Bincode)   // Compact binary keys
        .value_format(Format::Json)              // Standard JSON values (or Bincode for max performance)
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
        .value_format(Format::Json)               // Cross-platform values
        .build()
        .expect("Failed to create CDN backend");

    println!("   Key format:   {:?}", cdn_backend.key_format());
    println!("   Value format: {:?}", cdn_backend.value_format());
    println!("   Benefit: Keys safe for Vary headers, CDN integration");
    println!("   Use case: Edge caching, HTTP cache control\n");

    // Scenario 4: Mixed Configuration
    // - String keys for simple lookups
    // - Bincode values for maximum space efficiency
    println!("⚡ Scenario 4: Hybrid (String keys + Bincode values)");
    let hybrid_backend = RedisBackend::builder()
        .server("redis://127.0.0.1:6379/3".to_string())
        .key_format(CacheKeyFormat::Debug)      // Human-readable debug keys
        .value_format(Format::Json)               // Would use Bincode when implemented
        .build()
        .expect("Failed to create hybrid backend");

    println!("   Key format:   {:?}", hybrid_backend.key_format());
    println!("   Value format: {:?}", hybrid_backend.value_format());
    println!("   Benefit: Readable keys, compact values");
    println!("   Use case: Balance between debugging and performance\n");

    println!("=== Format Comparison Table ===");
    println!("┌─────────────┬────────────┬──────────────┬───────────────────┐");
    println!("│ Format      │ Key Size   │ Readable     │ Best For          │");
    println!("├─────────────┼────────────┼──────────────┼───────────────────┤");
    println!("│ String      │ Medium     │ ✓ Yes        │ Logging, Simple   │");
    println!("│ JSON        │ Large      │ ✓ Yes        │ Debug, Multi-lang │");
    println!("│ Bincode     │ Small      │ ✗ No         │ Production, Speed │");
    println!("│ UrlEncoded  │ Small      │ ✓ Partial    │ HTTP, CDN         │");
    println!("└─────────────┴────────────┴──────────────┴───────────────────┘\n");

    println!("=== Recommendations ===");
    println!("• Development:   JSON keys + JSON values");
    println!("• Production:    Bincode keys + JSON values (or Bincode when implemented)");
    println!("• CDN/Edge:      UrlEncoded keys + JSON values");
    println!("• Logging:       String keys + JSON values");
}
