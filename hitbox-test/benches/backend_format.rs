//! Benchmarks comparing serialization formats (JSON, Bincode, Postcard, RON, Rkyv) through
//! `Backend::set` and `Backend::get` on Moka.
//! All tests use PassthroughCompressor to isolate format performance
//!
//! - `write`, `read`: a body of repeated bytes and 2 headers.
//! - `write_json_body`, `read_json_body`: a valid JSON body made of the `data` items of
//!   `fixtures/reference_response.json`, and the 6 headers that `cache_future_reference` uses.
//! - `write_many_headers`, `read_many_headers`: the 1 KB JSON body and 20 headers.

use bytes::Bytes;
use criterion::measurement::WallTime;
use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};
use hitbox::{CacheKey, CachePolicy, CacheableResponse};
#[cfg(feature = "rkyv_format")]
use hitbox_backend::format::RkyvFormat;
use hitbox_backend::format::{BincodeFormat, Format, JsonFormat, PostcardFormat, RonFormat};
use hitbox_backend::{Backend, CacheBackend, PassthroughCompressor};
use hitbox_core::{CacheContext, CacheValue};
use hitbox_http::{BufferedBody, CacheableHttpResponse, SerializableHttpResponse};
use hitbox_moka::MokaBackend;
use http::Response;
use http_body_util::Empty;
use serde_json::{Value, from_str, to_string};
use tokio::runtime::Runtime;

type BenchBody = Empty<Bytes>;
type BenchResponse = CacheableHttpResponse<BenchBody>;

const RESPONSE_BODY: &str = include_str!("fixtures/reference_response.json");

const SIZES: [(&str, usize); 4] = [
    ("1KB", 1024),
    ("10KB", 10 * 1024),
    ("100KB", 100 * 1024),
    ("1MB", 1024 * 1024),
];

const PLAIN_HEADERS: [(&str, &str); 2] = [
    ("content-type", "application/json"),
    ("cache-control", "public, max-age=3600"),
];

const REFERENCE_HEADERS: [(&str, &str); 6] = [
    ("content-type", "application/json; charset=utf-8"),
    ("cache-control", "public, max-age=300"),
    ("x-request-id", "req-550e8400-e29b-41d4-a716-446655440000"),
    ("x-response-time", "42ms"),
    ("x-ratelimit-limit", "1000"),
    ("x-ratelimit-remaining", "999"),
];

const MANY_HEADERS: [(&str, &str); 20] = [
    ("content-type", "application/json; charset=utf-8"),
    ("cache-control", "public, max-age=300"),
    ("x-request-id", "req-550e8400-e29b-41d4-a716-446655440000"),
    ("x-response-time", "42ms"),
    ("x-ratelimit-limit", "1000"),
    ("x-ratelimit-remaining", "999"),
    ("date", "Wed, 04 Nov 2026 10:00:00 GMT"),
    ("etag", "\"5f3c-1a2b3c4d\""),
    ("vary", "accept-encoding, accept-language"),
    ("server", "nginx"),
    ("content-language", "en-US"),
    ("strict-transport-security", "max-age=31536000"),
    ("x-content-type-options", "nosniff"),
    ("x-frame-options", "DENY"),
    ("referrer-policy", "strict-origin-when-cross-origin"),
    ("access-control-allow-origin", "*"),
    ("set-cookie", "session=abc123; Path=/; HttpOnly; Secure"),
    ("set-cookie", "tenant=acme; Path=/; Secure"),
    ("x-cache", "MISS"),
    ("x-trace-id", "4bf92f3577b34da6a3ce929d0e0e4736"),
];

/// One response size in a group.
struct Case {
    size_name: &'static str,
    body_len: u64,
    response: SerializableHttpResponse,
}

/// Valid JSON of at least `size` bytes: the `data` items of the reference response, repeated.
fn json_body(size: usize) -> Bytes {
    let fixture: Value = from_str(RESPONSE_BODY).unwrap();
    let items: Vec<String> = fixture["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| to_string(item).unwrap())
        .collect();
    let mut body = String::from(r#"{"data":["#);
    for item in items.iter().cycle() {
        body.push_str(item);
        if body.len() + 2 >= size {
            break;
        }
        body.push(',');
    }
    body.push_str("]}");
    Bytes::from(body)
}

/// Generate test HTTP response with the specified body and headers
fn generate_response(body: Bytes, headers: &[(&str, &str)]) -> SerializableHttpResponse {
    let mut builder = Response::builder().status(200);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = builder
        .body(BufferedBody::<BenchBody>::Complete {
            data: Some(body),
            trailers: None,
        })
        .unwrap();

    let cacheable = CacheableHttpResponse::from_response(response);

    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        match cacheable.into_cached().await {
            CachePolicy::Cacheable(serializable) => serializable,
            CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    })
}

fn plain_cases() -> Vec<Case> {
    SIZES
        .into_iter()
        .map(|(size_name, size)| Case {
            size_name,
            body_len: size as u64,
            response: generate_response(Bytes::from(vec![b'x'; size]), &PLAIN_HEADERS),
        })
        .collect()
}

fn json_cases() -> Vec<Case> {
    SIZES
        .into_iter()
        .map(|(size_name, size)| {
            let body = json_body(size);
            println!("JSON body {size_name}: {} bytes", body.len());
            Case {
                size_name,
                body_len: body.len() as u64,
                response: generate_response(body, &REFERENCE_HEADERS),
            }
        })
        .collect()
}

fn many_headers_cases() -> Vec<Case> {
    let body = json_body(1024);
    vec![Case {
        size_name: "1KB",
        body_len: body.len() as u64,
        response: generate_response(body, &MANY_HEADERS),
    }]
}

fn moka_backend<S: Format + 'static>(format: S) -> MokaBackend<S, PassthroughCompressor> {
    MokaBackend::builder()
        .max_entries(10000)
        .value_format(format)
        .compressor(PassthroughCompressor)
        .build()
}

async fn bench_write_single<B>(backend: &B, serialized: &SerializableHttpResponse, key_num: u64)
where
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
    let _value: Option<CacheValue<SerializableHttpResponse>> =
        backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
}

fn bench_write<S: Format + 'static>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    runtime: &Runtime,
    name: &str,
    case: &Case,
    format: S,
) {
    let backend = moka_backend(format);
    group.bench_function(BenchmarkId::new(name, case.size_name), |b| {
        let mut counter = 0u64;
        b.to_async(runtime).iter(|| {
            let key_num = counter;
            counter = counter.wrapping_add(1);
            bench_write_single(&backend, &case.response, key_num)
        });
    });
}

fn bench_read<S: Format + 'static>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    runtime: &Runtime,
    name: &str,
    case: &Case,
    format: S,
) {
    let backend = moka_backend(format);
    runtime.block_on(async {
        for i in 0..100 {
            let key = CacheKey::from_str("bench", &format!("key-{}", i));
            let value = CacheValue::new(case.response.clone(), None, None);
            let mut ctx = CacheContext::default().boxed();
            backend
                .set::<BenchResponse>(&key, &value, &mut ctx)
                .await
                .unwrap();
        }
    });
    group.bench_function(BenchmarkId::new(name, case.size_name), |b| {
        let mut counter = 0u64;
        b.to_async(runtime).iter(|| {
            let key_num = counter % 100;
            counter = counter.wrapping_add(1);
            bench_read_single(&backend, key_num)
        });
    });
}

fn write_group(c: &mut Criterion, group_name: &str, cases: &[Case]) {
    let mut group = c.benchmark_group(group_name);
    let runtime = Runtime::new().unwrap();

    for case in cases {
        group.throughput(Throughput::Bytes(case.body_len));

        bench_write(&mut group, &runtime, "json", case, JsonFormat);
        bench_write(&mut group, &runtime, "bincode", case, BincodeFormat);
        bench_write(&mut group, &runtime, "postcard", case, PostcardFormat);
        bench_write(&mut group, &runtime, "ron", case, RonFormat);
        // Rkyv with 128KB buffer hint for better performance
        #[cfg(feature = "rkyv_format")]
        bench_write(
            &mut group,
            &runtime,
            "rkyv",
            case,
            RkyvFormat::with_buffer_hint(128 * 1024),
        );
    }

    group.finish();
}

fn read_group(c: &mut Criterion, group_name: &str, cases: &[Case]) {
    let mut group = c.benchmark_group(group_name);
    let runtime = Runtime::new().unwrap();

    for case in cases {
        group.throughput(Throughput::Bytes(case.body_len));

        bench_read(&mut group, &runtime, "json", case, JsonFormat);
        bench_read(&mut group, &runtime, "bincode", case, BincodeFormat);
        bench_read(&mut group, &runtime, "postcard", case, PostcardFormat);
        bench_read(&mut group, &runtime, "ron", case, RonFormat);
        // Rkyv with 128KB buffer hint for better performance
        #[cfg(feature = "rkyv_format")]
        bench_read(
            &mut group,
            &runtime,
            "rkyv",
            case,
            RkyvFormat::with_buffer_hint(128 * 1024),
        );
    }

    group.finish();
}

fn format_write_benchmark(c: &mut Criterion) {
    write_group(c, "write", &plain_cases());
}

fn format_read_benchmark(c: &mut Criterion) {
    read_group(c, "read", &plain_cases());
}

fn json_body_write_benchmark(c: &mut Criterion) {
    write_group(c, "write_json_body", &json_cases());
}

fn json_body_read_benchmark(c: &mut Criterion) {
    read_group(c, "read_json_body", &json_cases());
}

fn many_headers_write_benchmark(c: &mut Criterion) {
    write_group(c, "write_many_headers", &many_headers_cases());
}

fn many_headers_read_benchmark(c: &mut Criterion) {
    read_group(c, "read_many_headers", &many_headers_cases());
}

criterion_group!(
    benches,
    format_write_benchmark,
    format_read_benchmark,
    json_body_write_benchmark,
    json_body_read_benchmark,
    many_headers_write_benchmark,
    many_headers_read_benchmark
);
criterion_main!(benches);
