use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hitbox::{CacheKey, CacheableResponse};
use hitbox_backend::format::BincodeFormat;
use hitbox_backend::{Backend, CacheBackend, PassthroughCompressor};
use hitbox_core::{CacheContext, CacheValue};
use hitbox_feoxdb::FeOxDbBackend;
use hitbox_http::{BufferedBody, CacheableHttpResponse};
use hitbox_moka::MokaBackend;
use hitbox_redis::RedisBackend;
use http::Response;
use tempfile::TempDir;

// Use Empty as a placeholder body type since we only use BufferedBody::Complete
type BenchBody = http_body_util::Empty<Bytes>;
type BenchResponse = CacheableHttpResponse<BenchBody>;

/// Generate test HTTP response with specified body size
/// Returns SerializableHttpResponse (the cached form) for efficient cloning
fn generate_response(size_bytes: usize) -> hitbox_http::SerializableHttpResponse {
    let body = Bytes::from(vec![b'x'; size_bytes]);
    let response = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .header("cache-control", "public, max-age=3600")
        .body(BufferedBody::<BenchBody>::Complete {
            data: Some(body),
            trailers: None,
        })
        .unwrap();

    let cacheable = CacheableHttpResponse::from_response(response);

    // Convert to serializable form (what gets cached)
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        match cacheable.into_cached().await {
            hitbox::CachePolicy::Cacheable(serializable) => serializable,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    })
}

/// Benchmark write throughput (set operations)
async fn bench_write_single<B>(
    backend: &B,
    serialized: &hitbox_http::SerializableHttpResponse,
    key_num: u64,
) where
    B: Backend + CacheBackend,
{
    let key = CacheKey::from_str("bench", &format!("key-{}", key_num));
    let value = CacheValue::new(serialized.clone(), None, None, None);
    let mut ctx = CacheContext::default().boxed();
    backend
        .set::<BenchResponse>(&key, &value, &mut ctx)
        .await
        .unwrap();
}

/// Benchmark read throughput (get operations)
async fn bench_read_single<B>(backend: &B, key_num: u64)
where
    B: Backend + CacheBackend,
{
    let key = CacheKey::from_str("bench", &format!("key-{}", key_num));
    let mut ctx = CacheContext::default().boxed();
    let _value: Option<CacheValue<hitbox_http::SerializableHttpResponse>> =
        backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
}

/// Benchmark mixed read/write throughput
async fn bench_mixed_single<B>(
    backend: &B,
    serialized: &hitbox_http::SerializableHttpResponse,
    key_num: u64,
) where
    B: Backend + CacheBackend,
{
    let key = CacheKey::from_str("bench", &format!("key-{}", key_num));
    let value = CacheValue::new(serialized.clone(), None, None, None);
    let mut ctx = CacheContext::default().boxed();

    // Write
    backend
        .set::<BenchResponse>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Read back
    let _value: Option<CacheValue<hitbox_http::SerializableHttpResponse>> =
        backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
}

fn moka_backend_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("backend_write/moka");

    let sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64));

        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000; // Reuse 1000 keys to avoid memory exhaustion
                    counter = counter.wrapping_add(1);
                    bench_write_single(&backend, &response, key_num)
                });
            },
        );
    }

    group.finish();

    // Read benchmarks
    let mut group = c.benchmark_group("backend_read/moka");

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64));

        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        // Pre-populate cache
        runtime.block_on(async {
            for i in 0..1000 {
                let key = CacheKey::from_str("bench", &format!("key-{}", i));
                let value = CacheValue::new(response.clone(), None, None, None);
                let mut ctx = CacheContext::default().boxed();
                backend
                    .set::<BenchResponse>(&key, &value, &mut ctx)
                    .await
                    .unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000;
                    counter = counter.wrapping_add(1);
                    bench_read_single(&backend, key_num)
                });
            },
        );
    }

    group.finish();

    // Mixed benchmarks
    let mut group = c.benchmark_group("backend_mixed/moka");

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64 * 2));

        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000; // Reuse 1000 keys to avoid memory exhaustion
                    counter = counter.wrapping_add(1);
                    bench_mixed_single(&backend, &response, key_num)
                });
            },
        );
    }

    group.finish();
}

fn feoxdb_backend_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("backend_write/feoxdb");

    let sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64));

        // Create temp directory for this benchmark
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");

        let backend = FeOxDbBackend::builder()
            .path(db_path.to_string_lossy().to_string())
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build()
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000; // Reuse 1000 keys to avoid memory exhaustion
                    counter = counter.wrapping_add(1);
                    bench_write_single(&backend, &response, key_num)
                });
            },
        );
    }

    group.finish();

    // Read benchmarks
    let mut group = c.benchmark_group("backend_read/feoxdb");

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64));

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");

        let backend = FeOxDbBackend::builder()
            .path(db_path.to_string_lossy().to_string())
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build()
            .unwrap();

        // Pre-populate
        runtime.block_on(async {
            for i in 0..1000 {
                let key = CacheKey::from_str("bench", &format!("key-{}", i));
                let value = CacheValue::new(response.clone(), None, None, None);
                let mut ctx = CacheContext::default().boxed();
                backend
                    .set::<BenchResponse>(&key, &value, &mut ctx)
                    .await
                    .unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000;
                    counter = counter.wrapping_add(1);
                    bench_read_single(&backend, key_num)
                });
            },
        );
    }

    group.finish();

    // Mixed benchmarks
    let mut group = c.benchmark_group("backend_mixed/feoxdb");

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64 * 2));

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");

        let backend = FeOxDbBackend::builder()
            .path(db_path.to_string_lossy().to_string())
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build()
            .unwrap();

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000; // Reuse 1000 keys to avoid memory exhaustion
                    counter = counter.wrapping_add(1);
                    bench_mixed_single(&backend, &response, key_num)
                });
            },
        );
    }

    group.finish();
}

fn redis_backend_benchmarks(c: &mut Criterion) {
    use testcontainers::{ImageExt, runners::AsyncRunner};
    use testcontainers_modules::redis::Redis;

    let mut group = c.benchmark_group("backend_write/redis");

    let sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Check if REDIS_URL is set, otherwise use testcontainers with host networking
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| {
        // Start Redis container with host networking for accurate benchmarking
        // This avoids Docker bridge networking overhead
        let _container = runtime.block_on(async {
            Redis::default()
                .with_tag("7-alpine")
                .with_network("host")
                .start()
                .await
                .expect("Failed to start Redis container with host networking")
        });

        // Leak the container so it stays alive for the duration of benchmarks
        Box::leak(Box::new(_container));

        // With host networking, Redis is directly accessible on localhost:6379
        "redis://localhost:6379".to_string()
    });

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64));

        let backend = RedisBackend::builder()
            .connection(hitbox_redis::ConnectionMode::single(redis_url.clone()))
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build()
            .expect("Failed to create Redis backend");

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000; // Reuse 1000 keys to avoid memory exhaustion
                    counter = counter.wrapping_add(1);
                    bench_write_single(&backend, &response, key_num)
                });
            },
        );
    }

    group.finish();

    // Read benchmarks
    let mut group = c.benchmark_group("backend_read/redis");

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64));

        let backend = RedisBackend::builder()
            .connection(hitbox_redis::ConnectionMode::single(redis_url.clone()))
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build()
            .expect("Failed to create Redis backend");

        // Pre-populate
        runtime.block_on(async {
            for i in 0..1000 {
                let key = CacheKey::from_str("bench", &format!("key-{}", i));
                let value = CacheValue::new(response.clone(), None, None, None);
                let mut ctx = CacheContext::default().boxed();
                backend
                    .set::<BenchResponse>(&key, &value, &mut ctx)
                    .await
                    .unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000;
                    counter = counter.wrapping_add(1);
                    bench_read_single(&backend, key_num)
                });
            },
        );
    }

    group.finish();

    // Mixed benchmarks
    let mut group = c.benchmark_group("backend_mixed/redis");

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64 * 2));

        let backend = RedisBackend::builder()
            .connection(hitbox_redis::ConnectionMode::single(redis_url.clone()))
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build()
            .expect("Failed to create Redis backend");

        group.bench_with_input(
            BenchmarkId::new("bincode_passthrough", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 1000; // Reuse 1000 keys to avoid memory exhaustion
                    counter = counter.wrapping_add(1);
                    bench_mixed_single(&backend, &response, key_num)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    moka_backend_benchmarks,
    feoxdb_backend_benchmarks,
    redis_backend_benchmarks
);
criterion_main!(benches);
