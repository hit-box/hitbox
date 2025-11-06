use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use hitbox_backend::CacheKeyFormat;
use hitbox_core::CacheKey;
use std::hint::black_box;
use std::time::Instant;

fn create_small_key() -> CacheKey {
    CacheKey::from_slice(&[("method", Some("GET")), ("path", Some("/api/users"))])
}

fn create_medium_key() -> CacheKey {
    CacheKey::from_slice(&[
        ("method", Some("GET")),
        ("path", Some("/api/users/123")),
        ("tenant", Some("acme-corp")),
        ("user_id", Some("user-456")),
    ])
}

fn create_complex_key() -> CacheKey {
    CacheKey::from_slice(&[
        ("method", Some("POST")),
        (
            "path",
            Some("/api/v2/organizations/acme-corp/projects/project-123/resources"),
        ),
        ("tenant", Some("acme-corp")),
        ("user_id", Some("user-789")),
        ("accept", Some("application/json")),
        ("content-type", Some("application/json")),
        (
            "authorization",
            Some("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"),
        ),
        ("x-request-id", Some("req-abc-123-def-456")),
        ("query", Some("filter=active&sort=name&limit=50")),
    ])
}

fn create_very_complex_key() -> CacheKey {
    CacheKey::from_slice(&[
        ("method", Some("GET")),
        ("path", Some("/api/v3/search/advanced/results/paginated")),
        ("tenant", Some("enterprise-customer-2024")),
        ("user_id", Some("user-d4f5g6h7-i8j9-k0l1-m2n3-o4p5q6r7s8t9")),
        ("region", Some("us-east-1")),
        ("datacenter", Some("dc-01")),
        ("environment", Some("production")),
        ("service", Some("api-gateway")),
        ("version", Some("v3.2.1")),
        ("accept", Some("application/json; charset=utf-8")),
        ("accept-encoding", Some("gzip, deflate, br")),
        ("accept-language", Some("en-US,en;q=0.9")),
        ("cache-control", Some("no-cache")),
        ("x-correlation-id", Some("corr-abc123def456ghi789")),
        ("x-session-id", Some("sess-xyz987wvu654tsr321")),
    ])
}

fn bench_size_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("size_comparison");

    let test_cases = vec![
        ("small", create_small_key()),
        ("medium", create_medium_key()),
        ("complex", create_complex_key()),
        ("very_complex", create_very_complex_key()),
    ];

    for (name, key) in &test_cases {
        // Bitcode
        group.bench_with_input(BenchmarkId::new("bitcode", name), key, |b, key| {
            let format = CacheKeyFormat::Bitcode;
            b.iter(|| {
                let bytes = format.serialize(black_box(key)).unwrap();
                black_box(bytes.len())
            });
        });

        // UrlEncoded
        group.bench_with_input(BenchmarkId::new("urlencoded", name), key, |b, key| {
            let format = CacheKeyFormat::UrlEncoded;
            b.iter(|| {
                let bytes = format.serialize(black_box(key)).unwrap();
                black_box(bytes.len())
            });
        });
    }

    group.finish();

    // Print size and time comparison table
    println!("\n=== Size & Time Comparison ===\n");
    println!(
        "{:<20} {:>10} {:>12} {:>10} {:>10}",
        "Key Type", "Bitcode", "UrlEncoded", "Bit/U", "Bit(ns)"
    );
    println!(
        "{:<20} {:>10} {:>12} {:>10} {:>10}",
        "", "(bytes)", "(bytes)", "Ratio", ""
    );
    println!("{:-<72}", "");

    for (name, key) in test_cases {
        let bitcode_bytes = CacheKeyFormat::Bitcode.serialize(&key).unwrap();
        let urlencoded_bytes = CacheKeyFormat::UrlEncoded.serialize(&key).unwrap();

        let bitcode_ratio = bitcode_bytes.len() as f64 / urlencoded_bytes.len() as f64;

        // Measure time (average of 10000 iterations)
        let iterations = 10000;

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = black_box(CacheKeyFormat::Bitcode.serialize(black_box(&key)).unwrap());
        }
        let bitcode_time_ns = start.elapsed().as_nanos() / iterations;

        println!(
            "{:<20} {:>10} {:>12} {:>9.2}x {:>10}",
            name,
            bitcode_bytes.len(),
            urlencoded_bytes.len(),
            bitcode_ratio,
            bitcode_time_ns,
        );
    }
    println!();
}

criterion_group!(benches, bench_size_comparison);
criterion_main!(benches);
