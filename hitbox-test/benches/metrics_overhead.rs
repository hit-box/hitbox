//! Cost of recording backend metrics for small values.
//!
//! A Prometheus recorder is installed either way, so comparing a run with
//! `--features metrics` against one without shows what recording costs.

use std::thread;
use std::time::Duration;

use bytes::Bytes;
use criterion::{Criterion, criterion_group, criterion_main};
use hitbox::offload::OffloadManager;
use hitbox::{CacheKey, CacheableResponse};
use hitbox_backend::composition::policy::{CompositionPolicy, RefillPolicy};
use hitbox_backend::format::BincodeFormat;
use hitbox_backend::{CacheBackend, CompositionBackend, PassthroughCompressor};
use hitbox_core::{CacheContext, CacheValue};
use hitbox_http::{BufferedBody, CacheableHttpResponse, SerializableHttpResponse};
use hitbox_moka::MokaBackend;
use http::Response;
use metrics_exporter_prometheus::PrometheusBuilder;
use tokio::runtime::Runtime;

type BenchBody = http_body_util::Empty<Bytes>;
type BenchResponse = CacheableHttpResponse<BenchBody>;

/// The exporter's own upkeep interval when it runs the HTTP listener.
const UPKEEP_INTERVAL: Duration = Duration::from_secs(5);

async fn generate_response(size_bytes: usize) -> SerializableHttpResponse {
    let response = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .header("cache-control", "public, max-age=3600")
        .body(BufferedBody::<BenchBody>::Complete {
            data: Some(Bytes::from(vec![b'x'; size_bytes])),
            trailers: None,
        })
        .unwrap();

    match CacheableHttpResponse::from_response(response)
        .into_cached()
        .await
    {
        hitbox::CachePolicy::Cacheable(serializable) => serializable,
        hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
    }
}

fn moka() -> MokaBackend<BincodeFormat> {
    MokaBackend::builder()
        .max_entries(10000)
        .value_format(BincodeFormat)
        .compressor(PassthroughCompressor)
        .build()
}

fn bench_backend<B: CacheBackend>(
    c: &mut Criterion,
    runtime: &Runtime,
    name: &str,
    backend: B,
    value: &CacheValue<SerializableHttpResponse>,
) {
    let key = CacheKey::from_str("bench", "key1");
    runtime
        .block_on(async {
            let mut ctx = CacheContext::default().boxed();
            backend.set::<BenchResponse>(&key, value, &mut ctx).await
        })
        .unwrap();

    let mut group = c.benchmark_group("metrics_overhead");
    group.bench_function(format!("{name}_write"), |b| {
        b.to_async(runtime).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            backend
                .set::<BenchResponse>(&key, value, &mut ctx)
                .await
                .unwrap();
        });
    });
    group.bench_function(format!("{name}_read"), |b| {
        b.to_async(runtime).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            backend.get::<BenchResponse>(&key, &mut ctx).await.unwrap();
        });
    });
    group.finish();
}

fn bench_metrics_overhead(c: &mut Criterion) {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("install the Prometheus recorder");
    // Without upkeep, histogram samples pile up in the recorder for the whole run.
    thread::spawn(move || {
        loop {
            thread::sleep(UPKEEP_INTERVAL);
            handle.run_upkeep();
        }
    });

    let runtime = Runtime::new().unwrap();
    let value = CacheValue::new(runtime.block_on(generate_response(1024)), None, None);

    bench_backend(c, &runtime, "moka", moka(), &value);

    let composition = CompositionBackend::new(moka(), moka(), OffloadManager::default())
        .with_policy(CompositionPolicy::new().refill(RefillPolicy::Never));
    bench_backend(c, &runtime, "composition", composition, &value);
}

criterion_group!(benches, bench_metrics_overhead);
criterion_main!(benches);
