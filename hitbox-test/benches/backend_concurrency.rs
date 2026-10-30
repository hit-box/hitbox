use bytes::Bytes;
use hitbox::{CacheKey, CacheableResponse};
use hitbox_backend::format::BincodeFormat;
use hitbox_backend::{Backend, CacheBackend, PassthroughCompressor};
use hitbox_core::{CacheContext, CacheValue};
use hitbox_feoxdb::FeOxDbBackend;
use hitbox_http::{BufferedBody, CacheableHttpResponse};
use hitbox_moka::MokaBackend;
use hitbox_redis::RedisBackend;
use http::Response;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::redis::Redis;

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

/// Run concurrent write operations for a fixed duration
async fn concurrent_write_test<B>(
    backend: Arc<B>,
    response: hitbox_http::SerializableHttpResponse,
    num_tasks: usize,
    test_duration: Duration,
) -> (Duration, usize, usize)
where
    B: Backend + CacheBackend + Send + Sync + 'static,
{
    let start = Instant::now();
    let deadline = start + test_duration;
    let mut handles = Vec::new();

    for task_id in 0..num_tasks {
        let backend = Arc::clone(&backend);
        let response = response.clone();

        let handle = tokio::spawn(async move {
            let mut ops = 0;
            let mut errors = 0;

            while Instant::now() < deadline {
                let key_num = (task_id * 1000 + ops) % 1000;
                let key = CacheKey::from_str("bench", &format!("key-{}", key_num));
                let value = CacheValue::new(response.clone(), None, None);
                let mut ctx = CacheContext::default().boxed();

                if backend
                    .set::<BenchResponse>(&key, &value, &mut ctx)
                    .await
                    .is_err()
                {
                    errors += 1;
                }
                ops += 1;
            }
            (ops, errors)
        });
        handles.push(handle);
    }

    let mut total_ops = 0;
    let mut total_errors = 0;
    for handle in handles {
        let (ops, errors) = handle.await.unwrap();
        total_ops += ops;
        total_errors += errors;
    }

    let elapsed = start.elapsed();
    (elapsed, total_ops, total_errors)
}

/// Run concurrent read operations for a fixed duration
async fn concurrent_read_test<B>(
    backend: Arc<B>,
    num_tasks: usize,
    test_duration: Duration,
) -> (Duration, usize, usize)
where
    B: Backend + CacheBackend + Send + Sync + 'static,
{
    let start = Instant::now();
    let deadline = start + test_duration;
    let mut handles = Vec::new();

    for task_id in 0..num_tasks {
        let backend = Arc::clone(&backend);

        let handle = tokio::spawn(async move {
            let mut ops = 0;
            let mut errors = 0;

            while Instant::now() < deadline {
                let key_num = (task_id * 1000 + ops) % 1000;
                let key = CacheKey::from_str("bench", &format!("key-{}", key_num));
                let mut ctx = CacheContext::default().boxed();

                if backend.get::<BenchResponse>(&key, &mut ctx).await.is_err() {
                    errors += 1;
                }
                ops += 1;
            }
            (ops, errors)
        });
        handles.push(handle);
    }

    let mut total_ops = 0;
    let mut total_errors = 0;
    for handle in handles {
        let (ops, errors) = handle.await.unwrap();
        total_ops += ops;
        total_errors += errors;
    }

    let elapsed = start.elapsed();
    (elapsed, total_ops, total_errors)
}

/// Run concurrent mixed operations for a fixed duration
async fn concurrent_mixed_test<B>(
    backend: Arc<B>,
    response: hitbox_http::SerializableHttpResponse,
    num_tasks: usize,
    test_duration: Duration,
) -> (Duration, usize, usize)
where
    B: Backend + CacheBackend + Send + Sync + 'static,
{
    let start = Instant::now();
    let deadline = start + test_duration;
    let mut handles = Vec::new();

    for task_id in 0..num_tasks {
        let backend = Arc::clone(&backend);
        let response = response.clone();

        let handle = tokio::spawn(async move {
            let mut ops = 0;
            let mut errors = 0;

            while Instant::now() < deadline {
                let key_num = (task_id * 1000 + ops) % 1000;
                let key = CacheKey::from_str("bench", &format!("key-{}", key_num));
                let value = CacheValue::new(response.clone(), None, None);
                let mut ctx = CacheContext::default().boxed();

                // Write
                if backend
                    .set::<BenchResponse>(&key, &value, &mut ctx)
                    .await
                    .is_err()
                {
                    errors += 1;
                }

                // Read
                if backend.get::<BenchResponse>(&key, &mut ctx).await.is_err() {
                    errors += 1;
                }

                ops += 2; // Count both write and read
            }
            (ops, errors)
        });
        handles.push(handle);
    }

    let mut total_ops = 0;
    let mut total_errors = 0;
    for handle in handles {
        let (ops, errors) = handle.await.unwrap();
        total_ops += ops;
        total_errors += errors;
    }

    let elapsed = start.elapsed();
    (elapsed, total_ops, total_errors)
}

fn print_results(
    backend_name: &str,
    operation: &str,
    payload_size: &str,
    results: &[(usize, Duration, usize, usize)],
) {
    println!(
        "\n{} {} Concurrency Test ({})",
        backend_name, operation, payload_size
    );
    println!("{:-<90}", "");
    println!(
        "{:>12} {:>15} {:>20} {:>15} {:>20}",
        "Tasks", "Ops/sec", "Total Ops", "Errors", "Duration"
    );
    println!("{:-<90}", "");

    for (num_tasks, elapsed, total_ops, errors) in results {
        let ops_per_sec = (*total_ops as f64) / elapsed.as_secs_f64();
        let error_pct = if *total_ops > 0 {
            (*errors as f64 / *total_ops as f64) * 100.0
        } else {
            0.0
        };

        if *errors > 0 {
            println!(
                "{:>12} {:>15.0} {:>20} {:>12} ({:>5.1}%) {:>20.3}s",
                num_tasks,
                ops_per_sec,
                total_ops,
                errors,
                error_pct,
                elapsed.as_secs_f64()
            );
        } else {
            println!(
                "{:>12} {:>15.0} {:>20} {:>15} {:>20.3}s",
                num_tasks,
                ops_per_sec,
                total_ops,
                "-",
                elapsed.as_secs_f64()
            );
        }
    }
}

#[tokio::main]
async fn main() {
    let concurrency_levels = vec![1, 10, 100, 1000];

    // Read test duration from environment variable, default to 10 seconds
    let test_duration_secs: u64 = std::env::var("BENCH_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let test_duration = Duration::from_secs(test_duration_secs);

    let payload_sizes = [("1KB", 1024), ("10KB", 10 * 1024), ("100KB", 100 * 1024)];

    println!("Backend Concurrency Benchmark");
    println!("==============================");
    println!("Test duration per scenario: {}s", test_duration_secs);
    println!("Concurrency levels: {:?}", concurrency_levels);

    // Moka Backend Tests
    for (size_name, size_bytes) in &payload_sizes {
        let response = generate_response(*size_bytes).await;
        let backend = Arc::new(
            MokaBackend::builder()
                .max_entries(10000)
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build(),
        );

        // Write test
        let mut write_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) = concurrent_write_test(
                Arc::clone(&backend),
                response.clone(),
                num_tasks,
                test_duration,
            )
            .await;
            write_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("Moka", "Write", size_name, &write_results);

        // Pre-populate for read test
        for i in 0..1000 {
            let key = CacheKey::from_str("bench", &format!("key-{}", i));
            let value = CacheValue::new(response.clone(), None, None);
            let mut ctx = CacheContext::default().boxed();
            backend
                .set::<BenchResponse>(&key, &value, &mut ctx)
                .await
                .unwrap();
        }

        // Read test
        let mut read_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) =
                concurrent_read_test(Arc::clone(&backend), num_tasks, test_duration).await;
            read_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("Moka", "Read", size_name, &read_results);

        // Mixed test
        let mut mixed_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) = concurrent_mixed_test(
                Arc::clone(&backend),
                response.clone(),
                num_tasks,
                test_duration,
            )
            .await;
            mixed_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("Moka", "Mixed", size_name, &mixed_results);
    }

    // FeOxDB Backend Tests
    for (size_name, size_bytes) in &payload_sizes {
        let response = generate_response(*size_bytes).await;
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");

        let backend = Arc::new(
            FeOxDbBackend::builder()
                .path(db_path.to_string_lossy().to_string())
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build()
                .unwrap(),
        );

        // Write test
        let mut write_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) = concurrent_write_test(
                Arc::clone(&backend),
                response.clone(),
                num_tasks,
                test_duration,
            )
            .await;
            write_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("FeOxDB", "Write", size_name, &write_results);

        // Pre-populate for read test
        for i in 0..1000 {
            let key = CacheKey::from_str("bench", &format!("key-{}", i));
            let value = CacheValue::new(response.clone(), None, None);
            let mut ctx = CacheContext::default().boxed();
            let _ = backend.set::<BenchResponse>(&key, &value, &mut ctx).await;
        }

        // Read test
        let mut read_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) =
                concurrent_read_test(Arc::clone(&backend), num_tasks, test_duration).await;
            read_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("FeOxDB", "Read", size_name, &read_results);

        // Mixed test
        let mut mixed_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) = concurrent_mixed_test(
                Arc::clone(&backend),
                response.clone(),
                num_tasks,
                test_duration,
            )
            .await;
            mixed_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("FeOxDB", "Mixed", size_name, &mixed_results);
    }

    // Redis Backend Tests
    // Check if REDIS_URL is set, otherwise use testcontainers with host networking
    let _container: Option<ContainerAsync<Redis>> = None;
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| {
        // Start Redis container with host networking for accurate benchmarking
        // This avoids Docker bridge networking overhead
        let container = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                Redis::default()
                    .with_tag("7-alpine")
                    .with_network("host")
                    .start()
                    .await
                    .expect("Failed to start Redis container with host networking")
            })
        });

        // Leak the container so it stays alive for the duration of benchmarks
        Box::leak(Box::new(container));

        // With host networking, Redis is directly accessible on localhost:6379
        "redis://localhost:6379".to_string()
    });

    for (size_name, size_bytes) in &payload_sizes {
        let response = generate_response(*size_bytes).await;

        let backend = Arc::new(
            RedisBackend::builder()
                .connection(hitbox_redis::ConnectionMode::single(redis_url.clone()))
                .value_format(BincodeFormat)
                .compressor(PassthroughCompressor)
                .build()
                .expect("Failed to create Redis backend"),
        );

        // Write test
        let mut write_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) = concurrent_write_test(
                Arc::clone(&backend),
                response.clone(),
                num_tasks,
                test_duration,
            )
            .await;
            write_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("Redis", "Write", size_name, &write_results);

        // Pre-populate for read test
        for i in 0..1000 {
            let key = CacheKey::from_str("bench", &format!("key-{}", i));
            let value = CacheValue::new(response.clone(), None, None);
            let mut ctx = CacheContext::default().boxed();
            let _ = backend.set::<BenchResponse>(&key, &value, &mut ctx).await;
        }

        // Read test
        let mut read_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) =
                concurrent_read_test(Arc::clone(&backend), num_tasks, test_duration).await;
            read_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("Redis", "Read", size_name, &read_results);

        // Mixed test
        let mut mixed_results = Vec::new();
        for &num_tasks in &concurrency_levels {
            let (elapsed, total_ops, errors) = concurrent_mixed_test(
                Arc::clone(&backend),
                response.clone(),
                num_tasks,
                test_duration,
            )
            .await;
            mixed_results.push((num_tasks, elapsed, total_ops, errors));
        }
        print_results("Redis", "Mixed", size_name, &mixed_results);
    }

    println!("\n{:=<80}", "");
    println!("Benchmark Complete!");
}
