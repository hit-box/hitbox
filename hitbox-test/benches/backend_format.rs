//! Benchmarks comparing different serialization formats (JSON, Bincode, RON, Rkyv)
//! All tests use PassthroughCompressor to isolate format performance

use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hitbox::{CacheKey, CacheableResponse};
#[cfg(feature = "rkyv_format")]
use hitbox_backend::format::RkyvFormat;
use hitbox_backend::format::{BincodeFormat, JsonFormat, RonFormat};
use hitbox_backend::{Backend, CacheBackend, PassthroughCompressor};
use hitbox_core::{CacheContext, CacheValue};
use hitbox_http::{BufferedBody, CacheableHttpResponse};
use hitbox_moka::MokaBackend;
use http::Response;

type BenchBody = http_body_util::Empty<Bytes>;
type BenchResponse = CacheableHttpResponse<BenchBody>;

/// Generate test HTTP response with specified body size
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

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        match cacheable.into_cached().await {
            hitbox::CachePolicy::Cacheable(serializable) => serializable,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    })
}

async fn bench_write_single<B>(
    backend: &B,
    serialized: &hitbox_http::SerializableHttpResponse,
    key_num: u64,
) where
    B: Backend + CacheBackend,
{
    let key = CacheKey::from_str("bench", &format!("key-{}", key_num));
    let value = CacheValue::new(serialized.clone(), None, None);
    let mut ctx = CacheContext::default().boxed();
    backend
        .set::<BenchResponse>(&key, &value, &mut ctx)
        .await
        .unwrap();
}

async fn bench_read_single<B>(backend: &B, key_num: u64)
where
    B: Backend + CacheBackend,
{
    let key = CacheKey::from_str("bench", &format!("key-{}", key_num));
    let mut ctx = CacheContext::default().boxed();
    let _value: Option<CacheValue<hitbox_http::SerializableHttpResponse>> =
        backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
}

fn format_write_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");

    let sizes = [
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64));

        // JSON
        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(JsonFormat)
            .compressor(PassthroughCompressor)
            .build();

        group.bench_with_input(BenchmarkId::new("json", size_name), &size_bytes, |b, _| {
            let mut counter = 0u64;
            b.to_async(&runtime).iter(|| {
                let key_num = counter;
                counter = counter.wrapping_add(1);
                bench_write_single(&backend, &response, key_num)
            });
        });

        // Bincode
        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        group.bench_with_input(
            BenchmarkId::new("bincode", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter;
                    counter = counter.wrapping_add(1);
                    bench_write_single(&backend, &response, key_num)
                });
            },
        );

        // RON
        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(RonFormat)
            .compressor(PassthroughCompressor)
            .build();

        group.bench_with_input(BenchmarkId::new("ron", size_name), &size_bytes, |b, _| {
            let mut counter = 0u64;
            b.to_async(&runtime).iter(|| {
                let key_num = counter;
                counter = counter.wrapping_add(1);
                bench_write_single(&backend, &response, key_num)
            });
        });

        // Rkyv (if feature enabled) with 128KB buffer hint for better performance
        #[cfg(feature = "rkyv_format")]
        {
            let backend = MokaBackend::builder()
                .max_entries(10000)
                .value_format(RkyvFormat::with_buffer_hint(128 * 1024))
                .compressor(PassthroughCompressor)
                .build();

            group.bench_with_input(BenchmarkId::new("rkyv", size_name), &size_bytes, |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter;
                    counter = counter.wrapping_add(1);
                    bench_write_single(&backend, &response, key_num)
                });
            });
        }
    }

    group.finish();
}

fn format_read_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");

    let sizes = [
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for (size_name, size_bytes) in sizes {
        let response = generate_response(size_bytes);
        group.throughput(Throughput::Bytes(size_bytes as u64));

        // JSON - populate cache first
        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(JsonFormat)
            .compressor(PassthroughCompressor)
            .build();

        runtime.block_on(async {
            for i in 0..100 {
                let key = CacheKey::from_str("bench", &format!("key-{}", i));
                let value = CacheValue::new(response.clone(), None, None);
                let mut ctx = CacheContext::default().boxed();
                backend
                    .set::<BenchResponse>(&key, &value, &mut ctx)
                    .await
                    .unwrap();
            }
        });

        group.bench_with_input(BenchmarkId::new("json", size_name), &size_bytes, |b, _| {
            let mut counter = 0u64;
            b.to_async(&runtime).iter(|| {
                let key_num = counter % 100;
                counter = counter.wrapping_add(1);
                bench_read_single(&backend, key_num)
            });
        });

        // Bincode - populate cache first
        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        runtime.block_on(async {
            for i in 0..100 {
                let key = CacheKey::from_str("bench", &format!("key-{}", i));
                let value = CacheValue::new(response.clone(), None, None);
                let mut ctx = CacheContext::default().boxed();
                backend
                    .set::<BenchResponse>(&key, &value, &mut ctx)
                    .await
                    .unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::new("bincode", size_name),
            &size_bytes,
            |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 100;
                    counter = counter.wrapping_add(1);
                    bench_read_single(&backend, key_num)
                });
            },
        );

        // RON - populate cache first
        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(RonFormat)
            .compressor(PassthroughCompressor)
            .build();

        runtime.block_on(async {
            for i in 0..100 {
                let key = CacheKey::from_str("bench", &format!("key-{}", i));
                let value = CacheValue::new(response.clone(), None, None);
                let mut ctx = CacheContext::default().boxed();
                backend
                    .set::<BenchResponse>(&key, &value, &mut ctx)
                    .await
                    .unwrap();
            }
        });

        group.bench_with_input(BenchmarkId::new("ron", size_name), &size_bytes, |b, _| {
            let mut counter = 0u64;
            b.to_async(&runtime).iter(|| {
                let key_num = counter % 100;
                counter = counter.wrapping_add(1);
                bench_read_single(&backend, key_num)
            });
        });

        // Rkyv - populate cache first (if feature enabled) with 128KB buffer hint for better performance
        #[cfg(feature = "rkyv_format")]
        {
            let backend = MokaBackend::builder()
                .max_entries(10000)
                .value_format(RkyvFormat::with_buffer_hint(128 * 1024))
                .compressor(PassthroughCompressor)
                .build();

            runtime.block_on(async {
                for i in 0..100 {
                    let key = CacheKey::from_str("bench", &format!("key-{}", i));
                    let value = CacheValue::new(response.clone(), None, None);
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(&key, &value, &mut ctx)
                        .await
                        .unwrap();
                }
            });

            group.bench_with_input(BenchmarkId::new("rkyv", size_name), &size_bytes, |b, _| {
                let mut counter = 0u64;
                b.to_async(&runtime).iter(|| {
                    let key_num = counter % 100;
                    counter = counter.wrapping_add(1);
                    bench_read_single(&backend, key_num)
                });
            });
        }
    }

    group.finish();
}

criterion_group!(benches, format_write_benchmark, format_read_benchmark);
criterion_main!(benches);
