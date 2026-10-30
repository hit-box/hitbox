use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hitbox::offload::OffloadManager;
use hitbox::{CacheKey, CacheableResponse};
use hitbox_backend::composition::policy::{CompositionPolicy, RefillPolicy};
use hitbox_backend::format::BincodeFormat;
use hitbox_backend::{Backend, CacheBackend, CompositionBackend, PassthroughCompressor};
use hitbox_core::{CacheContext, CacheValue};
use hitbox_http::{BufferedBody, CacheableHttpResponse};
use hitbox_moka::MokaBackend;
use http::Response;
use std::sync::Arc;

// Use Empty as placeholder body type
type BenchBody = http_body_util::Empty<Bytes>;
type BenchResponse = CacheableHttpResponse<BenchBody>;

/// Generate test HTTP response with specified body size
async fn generate_response(size_bytes: usize) -> hitbox_http::SerializableHttpResponse {
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

    match cacheable.into_cached().await {
        hitbox::CachePolicy::Cacheable(serializable) => serializable,
        hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
    }
}

/// Benchmark direct Moka backend access (baseline)
fn bench_direct_moka(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("direct");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        let backend = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_write", size_name),
            &(&backend, &key, &value),
            |b, (backend, key, value)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(key, value, &mut ctx)
                        .await
                        .unwrap();
                });
            },
        );

        // Read benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_read", size_name),
            &(&backend, &key),
            |b, (backend, key)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(key, &mut ctx).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark single-level composition with concrete types
fn bench_composition_concrete(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("composition_concrete");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        let l1 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let l2 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let backend = CompositionBackend::new(l1, l2, OffloadManager::default())
            .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never));

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_write", size_name),
            &(&backend, &key, &value),
            |b, (backend, key, value)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(key, value, &mut ctx)
                        .await
                        .unwrap();
                });
            },
        );

        // Read benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_read", size_name),
            &(&backend, &key),
            |b, (backend, key)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(key, &mut ctx).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark composition with outer backend as dyn Backend
/// Note: CacheBackend is not object-safe, so we test with dyn Backend + CacheBackend calls
fn bench_composition_outer_dyn(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("composition_outer_dyn");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        let l1 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let l2 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let backend = CompositionBackend::new(l1, l2, OffloadManager::default())
            .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never));

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        let backend_clone = backend.clone();
        let key_clone = key.clone();
        let value_clone = value.clone();
        group.bench_function(BenchmarkId::new("moka_write", size_name), |b| {
            b.to_async(&runtime).iter(|| {
                let backend = backend_clone.clone();
                let key = key_clone.clone();
                let value = value_clone.clone();
                async move {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(&key, &value, &mut ctx)
                        .await
                        .unwrap();
                }
            });
        });

        // Read benchmark
        let backend_clone = backend.clone();
        let key_clone = key.clone();
        group.bench_function(BenchmarkId::new("moka_read", size_name), |b| {
            b.to_async(&runtime).iter(|| {
                let backend = backend_clone.clone();
                let key = key_clone.clone();
                async move {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
                }
            });
        });
    }

    group.finish();
}

/// Benchmark composition with inner backends as dyn Backend
fn bench_composition_inner_dyn(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("composition_inner_dyn");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        let l1: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let l2: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let backend = CompositionBackend::new(l1, l2, OffloadManager::default())
            .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never));

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        let backend_clone = backend.clone();
        let key_clone = key.clone();
        let value_clone = value.clone();
        group.bench_function(BenchmarkId::new("moka_write", size_name), |b| {
            b.to_async(&runtime).iter(|| {
                let backend = backend_clone.clone();
                let key = key_clone.clone();
                let value = value_clone.clone();
                async move {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(&key, &value, &mut ctx)
                        .await
                        .unwrap();
                }
            });
        });

        // Read benchmark
        let backend_clone = backend.clone();
        let key_clone = key.clone();
        group.bench_function(BenchmarkId::new("moka_read", size_name), |b| {
            b.to_async(&runtime).iter(|| {
                let backend = backend_clone.clone();
                let key = key_clone.clone();
                async move {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
                }
            });
        });
    }

    group.finish();
}

/// Benchmark composition with both outer and inner as dyn Backend
fn bench_composition_both_dyn(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("composition_both_dyn");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        let l1: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let l2: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let backend: Arc<dyn Backend + Send> = Arc::new(
            CompositionBackend::new(l1, l2, OffloadManager::default())
                .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never)),
        );

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        let backend_clone = backend.clone();
        let key_clone = key.clone();
        let value_clone = value.clone();
        group.bench_function(BenchmarkId::new("moka_write", size_name), |b| {
            b.to_async(&runtime).iter(|| {
                let backend = backend_clone.clone();
                let key = key_clone.clone();
                let value = value_clone.clone();
                async move {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(&key, &value, &mut ctx)
                        .await
                        .unwrap();
                }
            });
        });

        // Read benchmark
        let backend_clone = backend.clone();
        let key_clone = key.clone();
        group.bench_function(BenchmarkId::new("moka_read", size_name), |b| {
            b.to_async(&runtime).iter(|| {
                let backend = backend_clone.clone();
                let key = key_clone.clone();
                async move {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
                }
            });
        });
    }

    group.finish();
}

/// Benchmark 2-level nested composition with concrete types
fn bench_nested_2_concrete(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("nested_2_concrete");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        // Create L1 composition (Moka + Moka)
        let l1_inner1 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let l1_inner2 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let l1 = CompositionBackend::new(l1_inner1, l1_inner2, OffloadManager::default());

        // Create L2 (simple Moka)
        let l2 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        // Compose L1 composition with L2
        let backend = CompositionBackend::new(l1, l2, OffloadManager::default())
            .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never));

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_write", size_name),
            &(&backend, &key, &value),
            |b, (backend, key, value)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(key, value, &mut ctx)
                        .await
                        .unwrap();
                });
            },
        );

        // Read benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_read", size_name),
            &(&backend, &key),
            |b, (backend, key)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(key, &mut ctx).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 2-level nested composition with all dyn CacheBackend
fn bench_nested_2_dyn(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("nested_2_dyn");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        // Create L1 composition (Moka + Moka) as dyn
        let l1_inner1: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let l1_inner2: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let l1: Arc<dyn Backend + Send> = Arc::new(
            CompositionBackend::new(l1_inner1, l1_inner2, OffloadManager::default())
                .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never)),
        );

        // Create L2 (simple Moka) as dyn
        let l2: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        // Compose L1 composition with L2 as dyn
        let backend: Arc<dyn Backend + Send> = Arc::new(
            CompositionBackend::new(l1, l2, OffloadManager::default())
                .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never)),
        );

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        let backend_clone = backend.clone();
        let key_clone = key.clone();
        let value_clone = value.clone();
        group.bench_function(BenchmarkId::new("moka_write", size_name), |b| {
            b.to_async(&runtime).iter(|| {
                let backend = backend_clone.clone();
                let key = key_clone.clone();
                let value = value_clone.clone();
                async move {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(&key, &value, &mut ctx)
                        .await
                        .unwrap();
                }
            });
        });

        // Read benchmark
        let backend_clone = backend.clone();
        let key_clone = key.clone();
        group.bench_function(BenchmarkId::new("moka_read", size_name), |b| {
            b.to_async(&runtime).iter(|| {
                let backend = backend_clone.clone();
                let key = key_clone.clone();
                async move {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
                }
            });
        });
    }

    group.finish();
}

/// Benchmark 3-level nested composition with concrete types
fn bench_nested_3_concrete(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("nested_3_concrete");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        // Create deepest level (Moka + Moka)
        let l1_deep1 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let l1_deep2 = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let l1_middle = CompositionBackend::new(l1_deep1, l1_deep2, OffloadManager::default());

        // Create middle level
        let l2_middle = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let l1_top = CompositionBackend::new(l1_middle, l2_middle, OffloadManager::default());

        // Create top level
        let l2_top = MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build();

        let backend = CompositionBackend::new(l1_top, l2_top, OffloadManager::default());

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_write", size_name),
            &(&backend, &key, &value),
            |b, (backend, key, value)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(key, value, &mut ctx)
                        .await
                        .unwrap();
                });
            },
        );

        // Read benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_read", size_name),
            &(&backend, &key),
            |b, (backend, key)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(key, &mut ctx).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 3-level nested composition with all dyn CacheBackend
fn bench_nested_3_dyn(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("nested_3_dyn");

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    for (size_name, size_bytes) in &payload_sizes {
        group.throughput(Throughput::Bytes(*size_bytes as u64));

        // Create deepest level (Moka + Moka) as dyn
        let l1_deep1: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let l1_deep2: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let l1_middle: Arc<dyn Backend + Send> = Arc::new(
            CompositionBackend::new(l1_deep1, l1_deep2, OffloadManager::default())
                .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never)),
        );

        // Create middle level
        let l2_middle: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let l1_top: Arc<dyn Backend + Send> = Arc::new(
            CompositionBackend::new(l1_middle, l2_middle, OffloadManager::default())
                .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never)),
        );

        // Create top level
        let l2_top: Arc<dyn Backend + Send> = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        let backend: Arc<dyn Backend + Send> = Arc::new(
            CompositionBackend::new(l1_top, l2_top, OffloadManager::default())
                .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never)),
        );

        let response = runtime.block_on(generate_response(*size_bytes));
        let key = CacheKey::from_str("bench", "key1");
        let value = CacheValue::new(response.clone(), None, None);

        // Pre-populate for read benchmark
        runtime
            .block_on(async {
                let mut ctx = CacheContext::default().boxed();
                backend.set::<BenchResponse>(&key, &value, &mut ctx).await
            })
            .unwrap();

        // Write benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_write", size_name),
            &(&backend, &key, &value),
            |b, (backend, key, value)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend
                        .set::<BenchResponse>(key, value, &mut ctx)
                        .await
                        .unwrap();
                });
            },
        );

        // Read benchmark
        group.bench_with_input(
            BenchmarkId::new("moka_read", size_name),
            &(&backend, &key),
            |b, (backend, key)| {
                b.to_async(&runtime).iter(|| async {
                    let mut ctx = CacheContext::default().boxed();
                    backend.get::<BenchResponse>(key, &mut ctx).await.unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_direct_moka,
    bench_composition_concrete,
    bench_composition_outer_dyn,
    bench_composition_inner_dyn,
    bench_composition_both_dyn,
    bench_nested_2_concrete,
    bench_nested_2_dyn,
    bench_nested_3_concrete,
    bench_nested_3_dyn,
);

criterion_main!(benches);
