//! Cost of jq body predicates and extractors on one request, by body size and
//! by how many jq consumers inspect the body.
//!
//! Each benchmark evaluates a request whose body is already buffered, so the
//! time is JSON parsing plus running the filters:
//!
//! - `predicates/N`: a chain of N jq body predicates.
//! - `extractors/N`: a chain of N jq body extractors.
//! - `both/N`: N predicates, then N extractors on the subject they return,
//!   as cache policy evaluation runs them.
//!
//! The file also builds against hitbox-http 0.2 with only [`compat`] changed,
//! so a criterion baseline saved there can be compared with this version:
//!
//! ```text
//! # in a checkout of 0.2, with this file and its [[bench]] entry added
//! CRITERION_HOME=<dir> cargo bench -p hitbox-http --bench jq_body -- --save-baseline v0.2
//! # here
//! CRITERION_HOME=<dir> cargo bench -p hitbox-http --bench jq_body -- --baseline v0.2
//! ```

use std::time::Duration;

use bytes::Bytes;
use criterion::{BatchSize, BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use hitbox::Extractor;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::CacheableHttpRequest;
use hitbox_http::extractors::NeutralExtractor;
use hitbox_http::extractors::body::BodyExtractor;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::body::{BodyPredicate, JqExpression, JqOperation, Operation};
use http_body_util::Empty;
use serde_json::{Value, json};
use tokio::runtime::Builder;

type Request = CacheableHttpRequest<Empty<Bytes>>;
type BoxPredicate = Box<dyn Predicate<Subject = Request> + Send + Sync>;
type BoxExtractor = Box<dyn Extractor<Subject = Request> + Send + Sync>;

/// The only code that differs between hitbox-http versions.
mod compat {
    use bytes::Bytes;
    use hitbox_http::extractors::body::BodyExtraction;
    use hitbox_http::predicates::body::JqExpression;
    use hitbox_http::{BufferedBody, CacheableHttpRequest};

    use super::Request;

    /// A request whose body has already been buffered by an earlier consumer.
    pub fn request(body: &Bytes) -> Request {
        CacheableHttpRequest::from_request(
            http::Request::builder()
                .method("POST")
                .uri("/orders")
                .body(BufferedBody::Complete {
                    data: Some(body.clone()),
                    trailers: None,
                })
                .unwrap(),
        )
    }

    pub fn jq_extraction(filter: &str) -> BodyExtraction {
        BodyExtraction::Jq(JqExpression::compile(filter).unwrap())
    }
}

/// Body sizes: roughly 1 KB, 32 KB and 500 KB.
const SIZES: [(&str, usize); 3] = [("1KB", 5), ("32KB", 150), ("500KB", 2500)];

/// Cheap filters on different fields, so parsing dominates.
const PREDICATE_FILTERS: [&str; 3] = [".total", ".items[0].active", ".items[0].team.id"];
const EXTRACTOR_FILTERS: [&str; 3] = [".items[0].id", ".items[0].team.name", ".total"];

fn payload(items: usize) -> Bytes {
    let items: Vec<Value> = (0..items)
        .map(|i| {
            json!({
                "id": i,
                "name": format!("user-{i}"),
                "email": format!("user{i}@example.com"),
                "active": i % 2 == 0,
                "score": i as f64 * 1.5,
                "team": { "id": i % 10, "name": format!("team-{}", i % 10) },
                "tags": ["alpha", "beta", "gamma"],
            })
        })
        .collect();
    serde_json::to_vec(&json!({ "total": items.len(), "items": items }))
        .unwrap()
        .into()
}

fn jq_exists(filter: &str) -> Operation {
    Operation::Jq {
        filter: JqExpression::compile(filter).unwrap(),
        operation: JqOperation::Exist,
    }
}

/// A chain of `n` jq body predicates.
fn predicates(n: usize) -> BoxPredicate {
    let [a, b, c] = PREDICATE_FILTERS;
    let neutral = NeutralRequestPredicate::new();
    match n {
        1 => Box::new(neutral.body(jq_exists(a))),
        2 => Box::new(neutral.body(jq_exists(a)).body(jq_exists(b))),
        3 => Box::new(
            neutral
                .body(jq_exists(a))
                .body(jq_exists(b))
                .body(jq_exists(c)),
        ),
        _ => unreachable!("1 to 3 consumers"),
    }
}

/// A chain of `n` jq body extractors.
fn extractors(n: usize) -> BoxExtractor {
    let [a, b, c] = EXTRACTOR_FILTERS.map(compat::jq_extraction);
    let neutral = NeutralExtractor::new();
    match n {
        1 => Box::new(neutral.body(a)),
        2 => Box::new(neutral.body(a).body(b)),
        3 => Box::new(neutral.body(a).body(b).body(c)),
        _ => unreachable!("1 to 3 consumers"),
    }
}

fn bench_jq_body(c: &mut Criterion) {
    // One thread, so wake-ups never cross cores; run the bench pinned to a core.
    let rt = Builder::new_current_thread().build().unwrap();

    for (label, items) in SIZES {
        let body = payload(items);
        let mut group = c.benchmark_group(format!("jq_body/{label}"));
        group
            .warm_up_time(Duration::from_secs(3))
            .measurement_time(Duration::from_secs(8));
        if items >= 2500 {
            group
                .sampling_mode(SamplingMode::Flat)
                .sample_size(30)
                .measurement_time(Duration::from_secs(15));
        }

        for n in 1..=3 {
            let predicate = predicates(n);
            let extractor = extractors(n);

            group.bench_function(BenchmarkId::new("predicates", n), |b| {
                b.to_async(&rt).iter_batched(
                    || compat::request(&body),
                    |request| predicate.check(request),
                    BatchSize::SmallInput,
                )
            });

            group.bench_function(BenchmarkId::new("extractors", n), |b| {
                b.to_async(&rt).iter_batched(
                    || compat::request(&body),
                    |request| extractor.get(request),
                    BatchSize::SmallInput,
                )
            });

            group.bench_function(BenchmarkId::new("both", n), |b| {
                b.to_async(&rt).iter_batched(
                    || compat::request(&body),
                    |request| async {
                        let (PredicateResult::Cacheable(subject)
                        | PredicateResult::NonCacheable(subject)) = predicate.check(request).await;
                        extractor.get(subject).await
                    },
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }
}

criterion_group!(benches, bench_jq_body);
criterion_main!(benches);
