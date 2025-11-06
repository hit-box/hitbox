use bytes::Bytes;
use criterion::{Criterion, criterion_group, criterion_main};
use hitbox::predicate::Predicate;
use hitbox_http::CacheableHttpRequest;
use hitbox_http::predicates::{
    NeutralRequestPredicate,
    conditions::{NotPredicate, OrPredicate},
    request::{
        HeaderPredicate,
        header::Operation as HeaderOperation,
        method::MethodPredicate,
        path::PathPredicate,
        query::{Operation as QueryOperation, QueryPredicate},
    },
};
use http::{Method, Request};
use http_body_util::Empty;
use std::hint::black_box;

// ============================================================================
// Helper functions to create test requests
// ============================================================================

fn create_simple_request() -> Request<Empty<Bytes>> {
    Request::builder()
        .method(Method::GET)
        .uri("http://example.com/api/users/123")
        .header("content-type", "application/json")
        .header("authorization", "Bearer token123")
        .body(Empty::new())
        .unwrap()
}

fn create_complex_request() -> Request<Empty<Bytes>> {
    Request::builder()
        .method(Method::POST)
        .uri("http://example.com/api/v2/users/456/posts?filter=active&sort=date&limit=10")
        .header("content-type", "application/json")
        .header("authorization", "Bearer token456")
        .header("x-api-key", "secret123")
        .header("x-tenant-id", "tenant-a")
        .header("accept", "application/json")
        .header("user-agent", "Mozilla/5.0")
        .body(Empty::new())
        .unwrap()
}

// ============================================================================
// Single Predicate Benchmarks
// ============================================================================

fn bench_single_predicates(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_predicates");
    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("method_match", |b| {
        let predicate = NeutralRequestPredicate::new().method(Method::GET);
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("path_match", |b| {
        let predicate = NeutralRequestPredicate::new().path("/api/users/{user_id}".to_string());
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("header_single_match", |b| {
        let predicate = NeutralRequestPredicate::new().header(HeaderOperation::Eq(
            "content-type".parse().unwrap(),
            "application/json".parse().unwrap(),
        ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("header_exist", |b| {
        let predicate = NeutralRequestPredicate::new()
            .header(HeaderOperation::Exist("authorization".parse().unwrap()));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("query_match", |b| {
        let predicate = NeutralRequestPredicate::new().query(QueryOperation::Eq(
            "filter".to_string(),
            "active".to_string(),
        ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_complex_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.finish();
}

// ============================================================================
// Small Chain Benchmarks (2-4 predicates)
// ============================================================================

fn bench_small_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_chains");
    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("method_and_path", |b| {
        let predicate = NeutralRequestPredicate::new()
            .method(Method::GET)
            .path("/api/users/{user_id}".to_string());
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("method_path_header", |b| {
        let predicate = NeutralRequestPredicate::new()
            .method(Method::GET)
            .path("/api/users/{user_id}".to_string())
            .header(HeaderOperation::Eq(
                "content-type".parse().unwrap(),
                "application/json".parse().unwrap(),
            ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("method_path_header_query", |b| {
        let predicate = NeutralRequestPredicate::new()
            .method(Method::POST)
            .path("/api/v2/users/{user_id}/posts".to_string())
            .header(HeaderOperation::Eq(
                "content-type".parse().unwrap(),
                "application/json".parse().unwrap(),
            ))
            .query(QueryOperation::Eq(
                "filter".to_string(),
                "active".to_string(),
            ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_complex_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("or_two_methods", |b| {
        let base = NeutralRequestPredicate::new();
        let left = NeutralRequestPredicate::new().method(Method::GET);
        let right = NeutralRequestPredicate::new().method(Method::POST);
        let predicate = base.or(left, right);
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("not_method", |b| {
        let inner = NeutralRequestPredicate::new().method(Method::POST);
        let predicate =
            NeutralRequestPredicate::<CacheableHttpRequest<Empty<Bytes>>>::new().not(inner);
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.finish();
}

// ============================================================================
// Medium Chain Benchmarks (5-7 predicates)
// ============================================================================

fn bench_medium_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("medium_chains");
    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("method_and_four_headers", |b| {
        let predicate = NeutralRequestPredicate::new()
            .method(Method::POST)
            .header(HeaderOperation::Eq(
                "content-type".parse().unwrap(),
                "application/json".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "authorization".parse().unwrap(),
                "Bearer token456".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "x-api-key".parse().unwrap(),
                "secret123".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "x-tenant-id".parse().unwrap(),
                "tenant-a".parse().unwrap(),
            ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_complex_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("method_path_four_headers", |b| {
        let predicate = NeutralRequestPredicate::new()
            .method(Method::POST)
            .path("/api/v2/users/{user_id}/posts".to_string())
            .header(HeaderOperation::Eq(
                "content-type".parse().unwrap(),
                "application/json".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "authorization".parse().unwrap(),
                "Bearer token456".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "x-api-key".parse().unwrap(),
                "secret123".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "x-tenant-id".parse().unwrap(),
                "tenant-a".parse().unwrap(),
            ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_complex_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("full_chain_7_predicates", |b| {
        let predicate = NeutralRequestPredicate::new()
            .method(Method::POST)
            .path("/api/v2/users/{user_id}/posts".to_string())
            .header(HeaderOperation::Eq(
                "content-type".parse().unwrap(),
                "application/json".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "authorization".parse().unwrap(),
                "Bearer token456".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "x-api-key".parse().unwrap(),
                "secret123".parse().unwrap(),
            ))
            .query(QueryOperation::Eq(
                "filter".to_string(),
                "active".to_string(),
            ))
            .query(QueryOperation::Eq("sort".to_string(), "date".to_string()));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_complex_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.finish();
}

// ============================================================================
// Early Exit Benchmarks (test short-circuit behavior)
// ============================================================================

fn bench_early_exit(c: &mut Criterion) {
    let mut group = c.benchmark_group("early_exit");
    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("fail_first_predicate", |b| {
        // Method will fail immediately
        let predicate = NeutralRequestPredicate::new()
            .method(Method::DELETE) // Will fail on GET request
            .path("/api/users/{user_id}".to_string())
            .header(HeaderOperation::Eq(
                "content-type".parse().unwrap(),
                "application/json".parse().unwrap(),
            ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("fail_third_predicate", |b| {
        // First two pass, third fails
        let predicate = NeutralRequestPredicate::new()
            .method(Method::GET)
            .path("/api/users/{user_id}".to_string())
            .header(HeaderOperation::Eq(
                "x-nonexistent".parse().unwrap(),
                "value".parse().unwrap(),
            )); // Will fail
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("all_predicates_pass", |b| {
        // All predicates pass
        let predicate = NeutralRequestPredicate::new()
            .method(Method::GET)
            .path("/api/users/{user_id}".to_string())
            .header(HeaderOperation::Eq(
                "content-type".parse().unwrap(),
                "application/json".parse().unwrap(),
            ))
            .header(HeaderOperation::Eq(
                "authorization".parse().unwrap(),
                "Bearer token123".parse().unwrap(),
            ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.finish();
}

// ============================================================================
// Comparison: Different Predicate Types
// ============================================================================

fn bench_predicate_type_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("predicate_types");
    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("type_method", |b| {
        let predicate = NeutralRequestPredicate::new().method(Method::GET);
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("type_path", |b| {
        let predicate = NeutralRequestPredicate::new().path("/api/users/{user_id}".to_string());
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("type_header_eq", |b| {
        let predicate = NeutralRequestPredicate::new().header(HeaderOperation::Eq(
            "content-type".parse().unwrap(),
            "application/json".parse().unwrap(),
        ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("type_header_exist", |b| {
        let predicate = NeutralRequestPredicate::new()
            .header(HeaderOperation::Exist("authorization".parse().unwrap()));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_simple_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.bench_function("type_query", |b| {
        let predicate = NeutralRequestPredicate::new().query(QueryOperation::Eq(
            "filter".to_string(),
            "active".to_string(),
        ));
        b.to_async(&rt).iter(|| async {
            let req = CacheableHttpRequest::from_request(create_complex_request());
            black_box(predicate.check(black_box(req)).await)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_predicates,
    bench_small_chains,
    bench_medium_chains,
    bench_early_exit,
    bench_predicate_type_comparison
);
criterion_main!(benches);
