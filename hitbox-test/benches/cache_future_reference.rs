//! Reference benchmark for CacheFuture with realistic predicates, extractors, and payloads.
//!
//! This benchmark represents a typical API caching scenario:
//! - Request with path parameters, query params, headers, and JSON body (~2.5KB)
//! - Response with paginated JSON data (~5KB)
//! - Multiple predicates (method, path, headers)
//! - Multiple extractors (method, path, query, header with hash)
//! - Moka backend with Bincode format
//! - Real CacheFuture state machine
//!
//! Run with: cargo bench -p hitbox-test --bench cache_future_reference

use std::future::Ready;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use criterion::{Criterion, criterion_group, criterion_main};
use hitbox::CacheableResponse;
use hitbox::concurrency::NoopConcurrencyManager;
use hitbox::fsm::CacheFuture;
use hitbox::offload::OffloadManager;
use hitbox::policy::{EnabledCacheConfig, PolicyConfig};
use hitbox::predicate::Predicate;
use hitbox_backend::composition::policy::{
    CompositionPolicy, OptimisticParallelWritePolicy, RaceReadPolicy, RefillPolicy,
};
use hitbox_backend::format::BincodeFormat;
use hitbox_backend::{CacheBackend, CompositionBackend, PassthroughCompressor};
use hitbox_configuration::{Backend as ConfigBackend, ConfigEndpoints, Endpoint};
use hitbox_core::DisabledOffload;
use hitbox_core::Upstream;
use hitbox_http::extractors::MethodConfig;
use hitbox_http::extractors::NeutralExtractor;
use hitbox_http::extractors::body::{BodyExtraction, BodyExtractor, JqExtraction};
use hitbox_http::extractors::header::{
    Header, NameSelector as HeaderNameSelector, ValueExtractor as HeaderValueExtractor,
};
use hitbox_http::extractors::method::MethodExtractor;
use hitbox_http::extractors::path::PathExtractor;
use hitbox_http::extractors::query::{
    NameSelector as QueryNameSelector, Query, ValueExtractor as QueryValueExtractor,
};
use hitbox_http::extractors::transform::Transform;
use hitbox_http::predicates::body::{
    BodyPredicate, JqExpression, JqOperation, Operation as BodyOperation,
};
use hitbox_http::predicates::header::HeaderPredicate;
use hitbox_http::predicates::request::header::Operation as HeaderOperation;
use hitbox_http::predicates::request::method::MethodPredicate;
use hitbox_http::predicates::request::path::PathPredicate;
use hitbox_http::predicates::response::StatusCodePredicate as _;
use hitbox_http::predicates::response::header::Operation as ResponseHeaderOperation;
use hitbox_http::predicates::response::status::StatusClass;
use hitbox_http::predicates::{NeutralRequestPredicate, NeutralResponsePredicate};
use hitbox_http::{BufferedBody, CacheableHttpRequest, CacheableHttpResponse};
use hitbox_moka::MokaBackend;
use http::{Method, Request, Response};
use http_body_util::Empty;
use serde_json::json;

// Inner body type (what's inside BufferedBody)
type InnerBody = Empty<Bytes>;
// Request body type (BufferedBody wrapping the inner)
type ReqBody = BufferedBody<InnerBody>;
// Cacheable request/response types
type BenchRequest = CacheableHttpRequest<InnerBody>;
type BenchResponse = CacheableHttpResponse<InnerBody>;

// ============================================================================
// Fixture loading
// ============================================================================

const REQUEST_BODY: &str = include_str!("fixtures/reference_request.json");
const RESPONSE_BODY: &str = include_str!("fixtures/reference_response.json");
const REFERENCE_CONFIG: &str = include_str!("fixtures/reference_config.yaml");
const REFERENCE_CONFIG_BODY: &str = include_str!("fixtures/reference_config_body.yaml");

// ============================================================================
// Mock Upstream
// ============================================================================

/// Mock upstream that returns a fresh response each time
struct MockUpstream;

impl Upstream<BenchRequest> for MockUpstream {
    type Response = BenchResponse;
    type Future = Ready<Self::Response>;

    fn call(self, _req: BenchRequest) -> Self::Future {
        // Create a fresh response each time (no Clone needed)
        std::future::ready(CacheableHttpResponse::from_response(
            create_reference_response(),
        ))
    }
}

// ============================================================================
// Test data creation
// ============================================================================

/// Create a realistic API request with path params, query params, headers, and body
fn create_reference_request() -> Request<ReqBody> {
    Request::builder()
        .method(Method::POST)
        .uri("http://api.example.com/v1/users/12345/orders?include=items,customer&fields=id,status,total&expand=shipping")
        .header("content-type", "application/json")
        .header("authorization", "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U")
        .header("x-tenant-id", "tenant-abc-123")
        .header("x-request-id", "req-550e8400-e29b-41d4-a716-446655440000")
        .header("x-idempotency-key", "idem-key-unique-12345")
        .header("accept", "application/json")
        .header("accept-language", "en-US,en;q=0.9")
        .header("user-agent", "ApiClient/2.0 (Production)")
        .body(BufferedBody::Complete { data: Some(Bytes::from(REQUEST_BODY)), trailers: None })
        .unwrap()
}

/// Create a realistic API JSON response
fn create_reference_response() -> Response<ReqBody> {
    Response::builder()
        .status(200)
        .header("content-type", "application/json; charset=utf-8")
        .header("cache-control", "public, max-age=300")
        .header("x-request-id", "req-550e8400-e29b-41d4-a716-446655440000")
        .header("x-response-time", "42ms")
        .header("x-ratelimit-limit", "1000")
        .header("x-ratelimit-remaining", "999")
        .body(BufferedBody::Complete {
            data: Some(Bytes::from(RESPONSE_BODY)),
            trailers: None,
        })
        .unwrap()
}

// ============================================================================
// Predicate and Extractor setup
// ============================================================================

/// Create reference request predicates (method + path pattern + required headers) - static dispatch
fn create_request_predicates() -> impl Predicate<Subject = BenchRequest> + Send + Sync {
    NeutralRequestPredicate::new()
        .method(Method::POST)
        .path("/v1/users/{user_id}/orders".to_string())
        .header(HeaderOperation::Exist("authorization".parse().unwrap()))
        .header(HeaderOperation::Exist("x-tenant-id".parse().unwrap()))
        .header(HeaderOperation::Exist("x-idempotency-key".parse().unwrap()))
}

/// Create reference response predicates (cache successful 2xx responses) - static dispatch
fn create_response_predicates()
-> impl Predicate<Subject = <BenchResponse as CacheableResponse>::Subject> + Send + Sync {
    NeutralResponsePredicate::<InnerBody>::new()
        // Only cache 2xx successful responses
        .status(StatusClass::Success)
        // Require content-type header
        .header(ResponseHeaderOperation::Exist(
            "content-type".parse().unwrap(),
        ))
        // Require cache-control header (indicates response is cacheable)
        .header(ResponseHeaderOperation::Exist(
            "cache-control".parse().unwrap(),
        ))
}

/// Create reference extractors (method + path + query + headers with transforms) - static dispatch
fn create_extractors() -> impl hitbox::Extractor<Subject = BenchRequest> + Send + Sync {
    let base = NeutralExtractor::<InnerBody>::new();

    // Method extractor
    let with_method = base.method(MethodConfig::new());

    // Path extractor (extracts user_id)
    let with_path = with_method.path("/v1/users/{user_id}/orders");

    // Query extractors
    let with_query1 = Query::new_with(
        with_path,
        QueryNameSelector::Exact("include".to_string()),
        QueryValueExtractor::Full,
        Vec::new(),
    );
    let with_query2 = Query::new_with(
        with_query1,
        QueryNameSelector::Exact("fields".to_string()),
        QueryValueExtractor::Full,
        Vec::new(),
    );

    // Header: tenant-id (plain)
    let with_tenant = Header::new_with(
        with_query2,
        HeaderNameSelector::Exact("x-tenant-id".to_string()),
        HeaderValueExtractor::Full,
        Vec::new(),
    );

    // Header: authorization with hash transform (for privacy in cache key)
    let with_auth = Header::new_with(
        with_tenant,
        HeaderNameSelector::Exact("authorization".to_string()),
        HeaderValueExtractor::Full,
        vec![Transform::Hash],
    );

    // Header: idempotency key
    Header::new_with(
        with_auth,
        HeaderNameSelector::Exact("x-idempotency-key".to_string()),
        HeaderValueExtractor::Full,
        Vec::new(),
    )
}

/// Create cache policy (enabled with 5 min TTL)
fn create_policy() -> Arc<PolicyConfig> {
    Arc::new(PolicyConfig::Enabled(EnabledCacheConfig {
        ttl: Some(Duration::from_secs(300)),
        stale: None,
        policy: Default::default(),
        concurrency: None,
    }))
}

/// Extend request predicates with body jq predicate
fn create_request_predicates_with_body() -> impl Predicate<Subject = BenchRequest> + Send + Sync {
    let jq_filter = JqExpression::compile(".order.customer_id").unwrap();
    create_request_predicates().body(BodyOperation::Jq {
        filter: jq_filter,
        operation: JqOperation::Exist,
    })
}

/// Extend response predicates with body jq predicate
fn create_response_predicates_with_body()
-> impl Predicate<Subject = <BenchResponse as CacheableResponse>::Subject> + Send + Sync {
    let jq_filter = JqExpression::compile(".data | type").unwrap();
    create_response_predicates().body(BodyOperation::Jq {
        filter: jq_filter,
        operation: JqOperation::Eq(json!("array")),
    })
}

/// Extend extractors with body jq extractor
fn create_extractors_with_body() -> impl hitbox::Extractor<Subject = BenchRequest> + Send + Sync {
    let jq_extraction = JqExtraction::compile(
        "{customer_id: .order.customer_id, shipping: .order.shipping_method}",
    )
    .unwrap();
    create_extractors().body(BodyExtraction::Jq(jq_extraction))
}

// ============================================================================
// Benchmarks
// ============================================================================

/// Compare cache write operations: static vs dynamic backend dispatch
fn bench_compare_cache_write(c: &mut Criterion) {
    use hitbox::Extractor;
    use hitbox_core::{CacheContext, CacheValue};

    let mut group = c.benchmark_group("compare_cache_write");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // ===== Static backend setup =====
    let static_backend = MokaBackend::builder()
        .max_entries(10000)
        .value_format(BincodeFormat)
        .compressor(PassthroughCompressor)
        .build();

    // Generate cache key using extractors
    let extractors = create_extractors();
    let request = CacheableHttpRequest::from_request(create_reference_request());
    let (_, cache_key) = rt.block_on(async { extractors.get(request).await.into_cache_key() });

    // Generate cacheable response
    let response = CacheableHttpResponse::from_response(create_reference_response());
    let cached_response = rt.block_on(async {
        match response.into_cached().await {
            hitbox::CachePolicy::Cacheable(cached) => cached,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    });
    let cache_value = CacheValue::new(cached_response, None, None);

    // ===== Dynamic backend setup =====
    let config = load_config();
    let dyn_backend: DynBackend = config
        .backend
        .into_backend()
        .expect("Failed to create backend");

    // Static cache write benchmark
    let backend_write = static_backend.clone();
    let key_write = cache_key.clone();
    let value_write = cache_value.clone();
    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            backend_write
                .set::<BenchResponse>(&key_write, &value_write, &mut ctx)
                .await
                .unwrap();
        });
    });

    // Dynamic cache write benchmark
    let key_write = cache_key.clone();
    let value_write = cache_value.clone();
    group.bench_function("dynamic", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            dyn_backend
                .set::<BenchResponse>(&key_write, &value_write, &mut ctx)
                .await
                .unwrap();
        });
    });

    group.finish();
}

/// Compare cache read operations: static vs dynamic backend dispatch
fn bench_compare_cache_read(c: &mut Criterion) {
    use hitbox::Extractor;
    use hitbox_core::{CacheContext, CacheValue};

    let mut group = c.benchmark_group("compare_cache_read");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // ===== Static backend setup =====
    let static_backend = MokaBackend::builder()
        .max_entries(10000)
        .value_format(BincodeFormat)
        .compressor(PassthroughCompressor)
        .build();

    // Generate cache key using extractors
    let extractors = create_extractors();
    let request = CacheableHttpRequest::from_request(create_reference_request());
    let (_, cache_key) = rt.block_on(async { extractors.get(request).await.into_cache_key() });

    // Generate cacheable response
    let response = CacheableHttpResponse::from_response(create_reference_response());
    let cached_response = rt.block_on(async {
        match response.into_cached().await {
            hitbox::CachePolicy::Cacheable(cached) => cached,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    });
    let cache_value = CacheValue::new(cached_response, None, None);

    // Pre-populate static cache
    rt.block_on(async {
        let mut ctx = CacheContext::default().boxed();
        static_backend
            .set::<BenchResponse>(&cache_key, &cache_value, &mut ctx)
            .await
            .unwrap();
    });

    // ===== Dynamic backend setup =====
    let config = load_config();
    let dyn_backend: DynBackend = config
        .backend
        .into_backend()
        .expect("Failed to create backend");

    // Pre-populate dynamic cache
    rt.block_on(async {
        let mut ctx = CacheContext::default().boxed();
        dyn_backend
            .set::<BenchResponse>(&cache_key, &cache_value, &mut ctx)
            .await
            .unwrap();
    });

    // Static cache read benchmark
    let backend_read = static_backend.clone();
    let key_read = cache_key.clone();
    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            std::hint::black_box(
                backend_read
                    .get::<BenchResponse>(&key_read, &mut ctx)
                    .await
                    .unwrap(),
            );
        });
    });

    // Dynamic cache read benchmark
    let key_read = cache_key.clone();
    group.bench_function("dynamic", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            std::hint::black_box(
                dyn_backend
                    .get::<BenchResponse>(&key_read, &mut ctx)
                    .await
                    .unwrap(),
            );
        });
    });

    group.finish();
}

/// Compare CompositionBackend read operations: static vs dynamic dispatch
fn bench_compare_composition_read(c: &mut Criterion) {
    use hitbox::Extractor;
    use hitbox_backend::Backend;
    use hitbox_core::{CacheContext, CacheValue};

    let mut group = c.benchmark_group("compare_composition_read");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // ===== Static CompositionBackend setup (concrete types) =====
    let static_l1 = MokaBackend::builder()
        .max_entries(10000)
        .value_format(BincodeFormat)
        .compressor(PassthroughCompressor)
        .build();
    let static_l2 = MokaBackend::builder()
        .max_entries(10000)
        .value_format(BincodeFormat)
        .compressor(PassthroughCompressor)
        .build();
    // Use RaceReadPolicy for read benchmarks
    let static_composition =
        CompositionBackend::new(static_l1, static_l2, OffloadManager::default()).with_policy(
            CompositionPolicy::new()
                .read(RaceReadPolicy::new())
                .refill(RefillPolicy::Never),
        );

    // Generate cache key using extractors
    let extractors = create_extractors();
    let request = CacheableHttpRequest::from_request(create_reference_request());
    let (_, cache_key) = rt.block_on(async { extractors.get(request).await.into_cache_key() });

    // Generate cacheable response
    let response = CacheableHttpResponse::from_response(create_reference_response());
    let cached_response = rt.block_on(async {
        match response.into_cached().await {
            hitbox::CachePolicy::Cacheable(cached) => cached,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    });
    let cache_value = CacheValue::new(cached_response, None, None);

    // Pre-populate static composition cache
    rt.block_on(async {
        let mut ctx = CacheContext::default().boxed();
        static_composition
            .set::<BenchResponse>(&cache_key, &cache_value, &mut ctx)
            .await
            .unwrap();
    });

    // ===== Dynamic CompositionBackend setup (all levels are dyn Backend) =====
    let dyn_l1: Arc<dyn Backend + Send> = Arc::new(
        MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build(),
    );
    let dyn_l2: Arc<dyn Backend + Send> = Arc::new(
        MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build(),
    );
    // CompositionBackend with dyn Backend inner layers, wrapped as dyn Backend
    // Use RaceReadPolicy for read benchmarks
    let dyn_composition: Arc<dyn Backend + Send> = Arc::new(
        CompositionBackend::new(dyn_l1, dyn_l2, OffloadManager::default()).with_policy(
            CompositionPolicy::new()
                .read(RaceReadPolicy::new())
                .refill(RefillPolicy::Never),
        ),
    );

    // Pre-populate dynamic composition cache
    rt.block_on(async {
        let mut ctx = CacheContext::default().boxed();
        dyn_composition
            .set::<BenchResponse>(&cache_key, &cache_value, &mut ctx)
            .await
            .unwrap();
    });

    // Static composition read benchmark
    let key_read = cache_key.clone();
    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            std::hint::black_box(
                static_composition
                    .get::<BenchResponse>(&key_read, &mut ctx)
                    .await
                    .unwrap(),
            );
        });
    });

    // Dynamic composition read benchmark
    let key_read = cache_key.clone();
    group.bench_function("dynamic", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            std::hint::black_box(
                dyn_composition
                    .get::<BenchResponse>(&key_read, &mut ctx)
                    .await
                    .unwrap(),
            );
        });
    });

    group.finish();
}

/// Compare CompositionBackend write operations: static vs dynamic dispatch
fn bench_compare_composition_write(c: &mut Criterion) {
    use hitbox::Extractor;
    use hitbox_backend::Backend;
    use hitbox_core::{CacheContext, CacheValue};

    let mut group = c.benchmark_group("compare_composition_write");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // ===== Static CompositionBackend setup (concrete types) =====
    let static_l1 = MokaBackend::builder()
        .max_entries(10000)
        .value_format(BincodeFormat)
        .compressor(PassthroughCompressor)
        .build();
    let static_l2 = MokaBackend::builder()
        .max_entries(10000)
        .value_format(BincodeFormat)
        .compressor(PassthroughCompressor)
        .build();
    // Use OptimisticParallelWritePolicy for write benchmarks
    let static_composition =
        CompositionBackend::new(static_l1, static_l2, OffloadManager::default()).with_policy(
            CompositionPolicy::new()
                .write(OptimisticParallelWritePolicy::new())
                .refill(RefillPolicy::Never),
        );

    // Generate cache key using extractors
    let extractors = create_extractors();
    let request = CacheableHttpRequest::from_request(create_reference_request());
    let (_, cache_key) = rt.block_on(async { extractors.get(request).await.into_cache_key() });

    // Generate cacheable response
    let response = CacheableHttpResponse::from_response(create_reference_response());
    let cached_response = rt.block_on(async {
        match response.into_cached().await {
            hitbox::CachePolicy::Cacheable(cached) => cached,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    });
    let cache_value = CacheValue::new(cached_response, None, None);

    // ===== Dynamic CompositionBackend setup (all levels are dyn Backend) =====
    let dyn_l1: Arc<dyn Backend + Send> = Arc::new(
        MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build(),
    );
    let dyn_l2: Arc<dyn Backend + Send> = Arc::new(
        MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build(),
    );
    // CompositionBackend with dyn Backend inner layers, wrapped as dyn Backend
    // Use OptimisticParallelWritePolicy for write benchmarks
    let dyn_composition: Arc<dyn Backend + Send> = Arc::new(
        CompositionBackend::new(dyn_l1, dyn_l2, OffloadManager::default()).with_policy(
            CompositionPolicy::new()
                .write(OptimisticParallelWritePolicy::new())
                .refill(RefillPolicy::Never),
        ),
    );

    // Static composition write benchmark
    let key_write = cache_key.clone();
    let value_write = cache_value.clone();
    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            static_composition
                .set::<BenchResponse>(&key_write, &value_write, &mut ctx)
                .await
                .unwrap();
        });
    });

    // Dynamic composition write benchmark
    let key_write = cache_key.clone();
    let value_write = cache_value.clone();
    group.bench_function("dynamic", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = CacheContext::default().boxed();
            dyn_composition
                .set::<BenchResponse>(&key_write, &value_write, &mut ctx)
                .await
                .unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Dynamic Dispatch Setup (using hitbox-configuration)
// ============================================================================

/// Configuration parsed from YAML
#[derive(Debug, serde::Deserialize)]
struct BenchConfig {
    backend: ConfigBackend,
    endpoints: ConfigEndpoints,
}

/// Load configuration from YAML
fn load_config() -> BenchConfig {
    serde_saphyr::from_str(REFERENCE_CONFIG).expect("Failed to parse reference config")
}

/// Load body configuration from YAML
fn load_body_config() -> BenchConfig {
    serde_saphyr::from_str(REFERENCE_CONFIG_BODY).expect("Failed to parse body reference config")
}

/// Extract the first endpoint from a list of endpoint configurations.
fn first_endpoint(endpoints: ConfigEndpoints) -> Endpoint<InnerBody, InnerBody> {
    endpoints
        .into_iter()
        .next()
        .expect("endpoints list must not be empty")
        .into_endpoint()
        .expect("Failed to create endpoint")
}

// Type alias for dynamic backend (must match the blanket impl in hitbox_backend)
// Arc<dyn Backend + Send + 'static> implements CacheBackend, so CacheFuture can use it as B.
// However, CacheFuture::new takes Arc<B>, so we need Arc<Arc<dyn Backend + Send + 'static>>.
type DynBackend = Arc<dyn hitbox_backend::Backend + Send + 'static>;

// ============================================================================
// Comparison Benchmarks: Static vs Dynamic Dispatch
// ============================================================================

/// Compare request predicates: static vs dynamic dispatch
fn bench_compare_request_predicates(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_request_predicates");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Static dispatch predicates
    let static_predicates = create_request_predicates();

    // Dynamic dispatch predicates (from config)
    let config = load_config();
    let endpoint = first_endpoint(config.endpoints);
    let dynamic_predicates = endpoint.request_predicates;

    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let request = CacheableHttpRequest::from_request(create_reference_request());
            std::hint::black_box(static_predicates.check(request).await)
        });
    });

    group.bench_function("dynamic", |b| {
        let predicates = dynamic_predicates.clone();
        b.to_async(&rt).iter(|| async {
            let request = CacheableHttpRequest::from_request(create_reference_request());
            std::hint::black_box(predicates.check(request).await)
        });
    });

    group.finish();
}

/// Compare response predicates: static vs dynamic dispatch
fn bench_compare_response_predicates(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_response_predicates");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Static dispatch predicates
    let static_predicates = create_response_predicates();

    // Dynamic dispatch predicates (from config)
    let config = load_config();
    let endpoint = first_endpoint(config.endpoints);
    let dynamic_predicates = endpoint.response_predicates;

    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let response = CacheableHttpResponse::from_response(create_reference_response());
            std::hint::black_box(static_predicates.check(response).await)
        });
    });

    group.bench_function("dynamic", |b| {
        let predicates = dynamic_predicates.clone();
        b.to_async(&rt).iter(|| async {
            let response = CacheableHttpResponse::from_response(create_reference_response());
            std::hint::black_box(predicates.check(response).await)
        });
    });

    group.finish();
}

/// Compare extractors: static vs dynamic dispatch
fn bench_compare_extractors(c: &mut Criterion) {
    use hitbox::Extractor;

    let mut group = c.benchmark_group("compare_extractors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Static dispatch extractors
    let static_extractors = create_extractors();

    // Dynamic dispatch extractors (from config)
    let config = load_config();
    let endpoint = first_endpoint(config.endpoints);
    let dynamic_extractors = endpoint.extractors;

    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let request = CacheableHttpRequest::from_request(create_reference_request());
            std::hint::black_box(static_extractors.get(request).await)
        });
    });

    group.bench_function("dynamic", |b| {
        let extractors = dynamic_extractors.clone();
        b.to_async(&rt).iter(|| async {
            let request = CacheableHttpRequest::from_request(create_reference_request());
            std::hint::black_box(extractors.get(request).await)
        });
    });

    group.finish();
}

/// Compare full CacheFuture: static vs dynamic dispatch (cache hit)
fn bench_compare_cache_future_hit(c: &mut Criterion) {
    use hitbox::Extractor;
    use hitbox_core::{CacheContext, CacheValue};

    let mut group = c.benchmark_group("compare_cache_future_hit");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // ===== Static dispatch setup =====
    let static_backend = Arc::new(
        MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build(),
    );

    let static_request_predicates: Arc<dyn Predicate<Subject = BenchRequest> + Send + Sync> =
        Arc::new(create_request_predicates());
    let static_response_predicates: Arc<
        dyn Predicate<Subject = <BenchResponse as CacheableResponse>::Subject> + Send + Sync,
    > = Arc::new(create_response_predicates());
    let static_extractors: Arc<dyn hitbox::Extractor<Subject = BenchRequest> + Send + Sync> =
        Arc::new(create_extractors());
    let static_policy = create_policy();

    // Pre-populate static cache
    let request = CacheableHttpRequest::from_request(create_reference_request());
    let (_, static_cache_key) =
        rt.block_on(async { static_extractors.get(request).await.into_cache_key() });

    let response = CacheableHttpResponse::from_response(create_reference_response());
    let cached_response = rt.block_on(async {
        match response.into_cached().await {
            hitbox::CachePolicy::Cacheable(cached) => cached,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    });
    let cache_value = CacheValue::new(cached_response, None, None);

    rt.block_on(async {
        let mut ctx = CacheContext::default().boxed();
        static_backend
            .set::<BenchResponse>(&static_cache_key, &cache_value, &mut ctx)
            .await
            .unwrap();
    });

    // ===== Dynamic dispatch setup =====
    let config = load_config();
    let dyn_backend: DynBackend = config
        .backend
        .into_backend()
        .expect("Failed to create backend");
    let endpoint = first_endpoint(config.endpoints);
    let dyn_policy = Arc::clone(&endpoint.policy);

    // Pre-populate dynamic cache
    let request = CacheableHttpRequest::from_request(create_reference_request());
    let (_, dyn_cache_key) =
        rt.block_on(async { endpoint.extractors.get(request).await.into_cache_key() });

    let response = CacheableHttpResponse::from_response(create_reference_response());
    let cached_response = rt.block_on(async {
        match response.into_cached().await {
            hitbox::CachePolicy::Cacheable(cached) => cached,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    });
    let cache_value = CacheValue::new(cached_response, None, None);

    rt.block_on(async {
        let mut ctx = CacheContext::default().boxed();
        dyn_backend
            .set::<BenchResponse>(&dyn_cache_key, &cache_value, &mut ctx)
            .await
            .unwrap();
    });

    // Wrap for CacheFuture (needs Arc<Arc<...>>)
    let dyn_backend_arc: Arc<DynBackend> = Arc::new(dyn_backend);

    // ===== Benchmarks =====
    group.bench_function("static", |b| {
        let backend = static_backend.clone();
        let req_pred = static_request_predicates.clone();
        let res_pred = static_response_predicates.clone();
        let ext = static_extractors.clone();
        let policy = static_policy.clone();

        b.to_async(&rt).iter(|| {
            let backend = backend.clone();
            let req_pred = req_pred.clone();
            let res_pred = res_pred.clone();
            let ext = ext.clone();
            let policy = policy.clone();

            async move {
                let request = CacheableHttpRequest::from_request(create_reference_request());
                let upstream = MockUpstream;
                let cache_future = CacheFuture::new(
                    backend,
                    request,
                    upstream,
                    req_pred,
                    res_pred,
                    ext,
                    policy,
                    DisabledOffload,
                    NoopConcurrencyManager,
                );
                std::hint::black_box(cache_future.await)
            }
        });
    });

    group.bench_function("dynamic", |b| {
        let backend = dyn_backend_arc.clone();
        let req_pred = endpoint.request_predicates.clone();
        let res_pred = endpoint.response_predicates.clone();
        let ext = endpoint.extractors.clone();
        let policy = dyn_policy.clone();

        b.to_async(&rt).iter(|| {
            let backend = backend.clone();
            let req_pred = req_pred.clone();
            let res_pred = res_pred.clone();
            let ext = ext.clone();
            let policy = policy.clone();

            async move {
                let request = CacheableHttpRequest::from_request(create_reference_request());
                let upstream = MockUpstream;
                let cache_future = CacheFuture::new(
                    backend,
                    request,
                    upstream,
                    req_pred,
                    res_pred,
                    ext,
                    policy,
                    DisabledOffload,
                    NoopConcurrencyManager,
                );
                std::hint::black_box(cache_future.await)
            }
        });
    });

    group.finish();
}

/// Compare full CacheFuture: static vs dynamic dispatch (cache miss)
fn bench_compare_cache_future_miss(c: &mut Criterion) {
    use std::sync::atomic::{AtomicU64, Ordering};

    let mut group = c.benchmark_group("compare_cache_future_miss");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // ===== Static dispatch setup =====
    let static_backend = Arc::new(
        MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build(),
    );

    let static_request_predicates: Arc<dyn Predicate<Subject = BenchRequest> + Send + Sync> =
        Arc::new(create_request_predicates());
    let static_response_predicates: Arc<
        dyn Predicate<Subject = <BenchResponse as CacheableResponse>::Subject> + Send + Sync,
    > = Arc::new(create_response_predicates());
    let static_extractors: Arc<dyn hitbox::Extractor<Subject = BenchRequest> + Send + Sync> =
        Arc::new(create_extractors());
    let static_policy = create_policy();

    // ===== Dynamic dispatch setup =====
    let config = load_config();
    let dyn_backend: DynBackend = config
        .backend
        .into_backend()
        .expect("Failed to create backend");
    let endpoint = first_endpoint(config.endpoints);
    let dyn_policy = Arc::clone(&endpoint.policy);
    let dyn_backend_arc: Arc<DynBackend> = Arc::new(dyn_backend);

    // Use atomic counters for unique request IDs
    static STATIC_COUNTER: AtomicU64 = AtomicU64::new(0);
    static DYNAMIC_COUNTER: AtomicU64 = AtomicU64::new(1_000_000);

    // ===== Benchmarks =====
    group.bench_function("static", |b| {
        let backend = static_backend.clone();
        let req_pred = static_request_predicates.clone();
        let res_pred = static_response_predicates.clone();
        let ext = static_extractors.clone();
        let policy = static_policy.clone();

        b.to_async(&rt).iter(|| {
            let backend = backend.clone();
            let req_pred = req_pred.clone();
            let res_pred = res_pred.clone();
            let ext = ext.clone();
            let policy = policy.clone();
            let unique_id = STATIC_COUNTER.fetch_add(1, Ordering::Relaxed);

            async move {
                let request: Request<ReqBody> = Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "http://api.example.com/v1/users/{}/orders?include=items,customer&fields=id,status,total&expand=shipping",
                        unique_id
                    ))
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer token")
                    .header("x-tenant-id", "tenant-abc-123")
                    .header("x-idempotency-key", format!("idem-{}", unique_id))
                    .body(BufferedBody::Complete { data: Some(Bytes::from(REQUEST_BODY)), trailers: None })
                    .unwrap();
                let request = CacheableHttpRequest::from_request(request);
                let upstream = MockUpstream;
                let cache_future =
                    CacheFuture::new(backend, request, upstream, req_pred, res_pred, ext, policy, DisabledOffload, NoopConcurrencyManager);
                std::hint::black_box(cache_future.await)
            }
        });
    });

    group.bench_function("dynamic", |b| {
        let backend = dyn_backend_arc.clone();
        let req_pred = endpoint.request_predicates.clone();
        let res_pred = endpoint.response_predicates.clone();
        let ext = endpoint.extractors.clone();
        let policy = dyn_policy.clone();

        b.to_async(&rt).iter(|| {
            let backend = backend.clone();
            let req_pred = req_pred.clone();
            let res_pred = res_pred.clone();
            let ext = ext.clone();
            let policy = policy.clone();
            let unique_id = DYNAMIC_COUNTER.fetch_add(1, Ordering::Relaxed);

            async move {
                let request: Request<ReqBody> = Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "http://api.example.com/v1/users/{}/orders?include=items,customer&fields=id,status,total&expand=shipping",
                        unique_id
                    ))
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer token")
                    .header("x-tenant-id", "tenant-abc-123")
                    .header("x-idempotency-key", format!("idem-{}", unique_id))
                    .body(BufferedBody::Complete { data: Some(Bytes::from(REQUEST_BODY)), trailers: None })
                    .unwrap();
                let request = CacheableHttpRequest::from_request(request);
                let upstream = MockUpstream;
                let cache_future =
                    CacheFuture::new(backend, request, upstream, req_pred, res_pred, ext, policy, DisabledOffload, NoopConcurrencyManager);
                std::hint::black_box(cache_future.await)
            }
        });
    });

    group.finish();
}

// ============================================================================
// Body Comparison Benchmarks: Static vs Dynamic Dispatch (with jq)
// ============================================================================

/// Compare body request predicates: static vs dynamic dispatch
fn bench_compare_body_request_predicates(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_body_request_predicates");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Static dispatch predicates with body
    let static_predicates = create_request_predicates_with_body();

    // Dynamic dispatch predicates (from body config)
    let config = load_body_config();
    let endpoint = first_endpoint(config.endpoints);
    let dynamic_predicates = endpoint.request_predicates;

    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let request = CacheableHttpRequest::from_request(create_reference_request());
            std::hint::black_box(static_predicates.check(request).await)
        });
    });

    group.bench_function("dynamic", |b| {
        let predicates = dynamic_predicates.clone();
        b.to_async(&rt).iter(|| async {
            let request = CacheableHttpRequest::from_request(create_reference_request());
            std::hint::black_box(predicates.check(request).await)
        });
    });

    group.finish();
}

/// Compare body response predicates: static vs dynamic dispatch
fn bench_compare_body_response_predicates(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_body_response_predicates");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Static dispatch predicates with body
    let static_predicates = create_response_predicates_with_body();

    // Dynamic dispatch predicates (from body config)
    let config = load_body_config();
    let endpoint = first_endpoint(config.endpoints);
    let dynamic_predicates = endpoint.response_predicates;

    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let response = CacheableHttpResponse::from_response(create_reference_response());
            std::hint::black_box(static_predicates.check(response).await)
        });
    });

    group.bench_function("dynamic", |b| {
        let predicates = dynamic_predicates.clone();
        b.to_async(&rt).iter(|| async {
            let response = CacheableHttpResponse::from_response(create_reference_response());
            std::hint::black_box(predicates.check(response).await)
        });
    });

    group.finish();
}

/// Compare body extractors: static vs dynamic dispatch
fn bench_compare_body_extractors(c: &mut Criterion) {
    use hitbox::Extractor;

    let mut group = c.benchmark_group("compare_body_extractors");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Static dispatch extractors with body
    let static_extractors = create_extractors_with_body();

    // Dynamic dispatch extractors (from body config)
    let config = load_body_config();
    let endpoint = first_endpoint(config.endpoints);
    let dynamic_extractors = endpoint.extractors;

    group.bench_function("static", |b| {
        b.to_async(&rt).iter(|| async {
            let request = CacheableHttpRequest::from_request(create_reference_request());
            std::hint::black_box(static_extractors.get(request).await)
        });
    });

    group.bench_function("dynamic", |b| {
        let extractors = dynamic_extractors.clone();
        b.to_async(&rt).iter(|| async {
            let request = CacheableHttpRequest::from_request(create_reference_request());
            std::hint::black_box(extractors.get(request).await)
        });
    });

    group.finish();
}

/// Compare body full CacheFuture: static vs dynamic dispatch (cache hit)
fn bench_compare_body_cache_future_hit(c: &mut Criterion) {
    use hitbox::Extractor;
    use hitbox_core::{CacheContext, CacheValue};

    let mut group = c.benchmark_group("compare_body_cache_future_hit");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // ===== Static dispatch setup with body =====
    let static_backend = Arc::new(
        MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build(),
    );

    let static_request_predicates: Arc<dyn Predicate<Subject = BenchRequest> + Send + Sync> =
        Arc::new(create_request_predicates_with_body());
    let static_response_predicates: Arc<
        dyn Predicate<Subject = <BenchResponse as CacheableResponse>::Subject> + Send + Sync,
    > = Arc::new(create_response_predicates_with_body());
    let static_extractors: Arc<dyn hitbox::Extractor<Subject = BenchRequest> + Send + Sync> =
        Arc::new(create_extractors_with_body());
    let static_policy = create_policy();

    // Pre-populate static cache
    let request = CacheableHttpRequest::from_request(create_reference_request());
    let (_, static_cache_key) =
        rt.block_on(async { static_extractors.get(request).await.into_cache_key() });

    let response = CacheableHttpResponse::from_response(create_reference_response());
    let cached_response = rt.block_on(async {
        match response.into_cached().await {
            hitbox::CachePolicy::Cacheable(cached) => cached,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    });
    let cache_value = CacheValue::new(cached_response, None, None);

    rt.block_on(async {
        let mut ctx = CacheContext::default().boxed();
        static_backend
            .set::<BenchResponse>(&static_cache_key, &cache_value, &mut ctx)
            .await
            .unwrap();
    });

    // ===== Dynamic dispatch setup with body =====
    let config = load_body_config();
    let dyn_backend: DynBackend = config
        .backend
        .into_backend()
        .expect("Failed to create backend");
    let endpoint = first_endpoint(config.endpoints);
    let dyn_policy = Arc::clone(&endpoint.policy);

    // Pre-populate dynamic cache
    let request = CacheableHttpRequest::from_request(create_reference_request());
    let (_, dyn_cache_key) =
        rt.block_on(async { endpoint.extractors.get(request).await.into_cache_key() });

    let response = CacheableHttpResponse::from_response(create_reference_response());
    let cached_response = rt.block_on(async {
        match response.into_cached().await {
            hitbox::CachePolicy::Cacheable(cached) => cached,
            hitbox::CachePolicy::NonCacheable(_) => panic!("Response should be cacheable"),
        }
    });
    let cache_value = CacheValue::new(cached_response, None, None);

    rt.block_on(async {
        let mut ctx = CacheContext::default().boxed();
        dyn_backend
            .set::<BenchResponse>(&dyn_cache_key, &cache_value, &mut ctx)
            .await
            .unwrap();
    });

    let dyn_backend_arc: Arc<DynBackend> = Arc::new(dyn_backend);

    // ===== Benchmarks =====
    group.bench_function("static", |b| {
        let backend = static_backend.clone();
        let req_pred = static_request_predicates.clone();
        let res_pred = static_response_predicates.clone();
        let ext = static_extractors.clone();
        let policy = static_policy.clone();

        b.to_async(&rt).iter(|| {
            let backend = backend.clone();
            let req_pred = req_pred.clone();
            let res_pred = res_pred.clone();
            let ext = ext.clone();
            let policy = policy.clone();

            async move {
                let request = CacheableHttpRequest::from_request(create_reference_request());
                let upstream = MockUpstream;
                let cache_future = CacheFuture::new(
                    backend,
                    request,
                    upstream,
                    req_pred,
                    res_pred,
                    ext,
                    policy,
                    DisabledOffload,
                    NoopConcurrencyManager,
                );
                std::hint::black_box(cache_future.await)
            }
        });
    });

    group.bench_function("dynamic", |b| {
        let backend = dyn_backend_arc.clone();
        let req_pred = endpoint.request_predicates.clone();
        let res_pred = endpoint.response_predicates.clone();
        let ext = endpoint.extractors.clone();
        let policy = dyn_policy.clone();

        b.to_async(&rt).iter(|| {
            let backend = backend.clone();
            let req_pred = req_pred.clone();
            let res_pred = res_pred.clone();
            let ext = ext.clone();
            let policy = policy.clone();

            async move {
                let request = CacheableHttpRequest::from_request(create_reference_request());
                let upstream = MockUpstream;
                let cache_future = CacheFuture::new(
                    backend,
                    request,
                    upstream,
                    req_pred,
                    res_pred,
                    ext,
                    policy,
                    DisabledOffload,
                    NoopConcurrencyManager,
                );
                std::hint::black_box(cache_future.await)
            }
        });
    });

    group.finish();
}

/// Compare body full CacheFuture: static vs dynamic dispatch (cache miss)
fn bench_compare_body_cache_future_miss(c: &mut Criterion) {
    use std::sync::atomic::{AtomicU64, Ordering};

    let mut group = c.benchmark_group("compare_body_cache_future_miss");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // ===== Static dispatch setup with body =====
    let static_backend = Arc::new(
        MokaBackend::builder()
            .max_entries(10000)
            .value_format(BincodeFormat)
            .compressor(PassthroughCompressor)
            .build(),
    );

    let static_request_predicates: Arc<dyn Predicate<Subject = BenchRequest> + Send + Sync> =
        Arc::new(create_request_predicates_with_body());
    let static_response_predicates: Arc<
        dyn Predicate<Subject = <BenchResponse as CacheableResponse>::Subject> + Send + Sync,
    > = Arc::new(create_response_predicates_with_body());
    let static_extractors: Arc<dyn hitbox::Extractor<Subject = BenchRequest> + Send + Sync> =
        Arc::new(create_extractors_with_body());
    let static_policy = create_policy();

    // ===== Dynamic dispatch setup with body =====
    let config = load_body_config();
    let dyn_backend: DynBackend = config
        .backend
        .into_backend()
        .expect("Failed to create backend");
    let endpoint = first_endpoint(config.endpoints);
    let dyn_policy = Arc::clone(&endpoint.policy);
    let dyn_backend_arc: Arc<DynBackend> = Arc::new(dyn_backend);

    // Use atomic counters for unique request IDs
    static BODY_STATIC_COUNTER: AtomicU64 = AtomicU64::new(30_000_000);
    static BODY_DYNAMIC_COUNTER: AtomicU64 = AtomicU64::new(40_000_000);

    // ===== Benchmarks =====
    group.bench_function("static", |b| {
        let backend = static_backend.clone();
        let req_pred = static_request_predicates.clone();
        let res_pred = static_response_predicates.clone();
        let ext = static_extractors.clone();
        let policy = static_policy.clone();

        b.to_async(&rt).iter(|| {
            let backend = backend.clone();
            let req_pred = req_pred.clone();
            let res_pred = res_pred.clone();
            let ext = ext.clone();
            let policy = policy.clone();
            let unique_id = BODY_STATIC_COUNTER.fetch_add(1, Ordering::Relaxed);

            async move {
                let request: Request<ReqBody> = Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "http://api.example.com/v1/users/{}/orders?include=items,customer&fields=id,status,total&expand=shipping",
                        unique_id
                    ))
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer token")
                    .header("x-tenant-id", "tenant-abc-123")
                    .header("x-idempotency-key", format!("idem-{}", unique_id))
                    .body(BufferedBody::Complete { data: Some(Bytes::from(REQUEST_BODY)), trailers: None })
                    .unwrap();
                let request = CacheableHttpRequest::from_request(request);
                let upstream = MockUpstream;
                let cache_future =
                    CacheFuture::new(backend, request, upstream, req_pred, res_pred, ext, policy, DisabledOffload, NoopConcurrencyManager);
                std::hint::black_box(cache_future.await)
            }
        });
    });

    group.bench_function("dynamic", |b| {
        let backend = dyn_backend_arc.clone();
        let req_pred = endpoint.request_predicates.clone();
        let res_pred = endpoint.response_predicates.clone();
        let ext = endpoint.extractors.clone();
        let policy = dyn_policy.clone();

        b.to_async(&rt).iter(|| {
            let backend = backend.clone();
            let req_pred = req_pred.clone();
            let res_pred = res_pred.clone();
            let ext = ext.clone();
            let policy = policy.clone();
            let unique_id = BODY_DYNAMIC_COUNTER.fetch_add(1, Ordering::Relaxed);

            async move {
                let request: Request<ReqBody> = Request::builder()
                    .method(Method::POST)
                    .uri(format!(
                        "http://api.example.com/v1/users/{}/orders?include=items,customer&fields=id,status,total&expand=shipping",
                        unique_id
                    ))
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer token")
                    .header("x-tenant-id", "tenant-abc-123")
                    .header("x-idempotency-key", format!("idem-{}", unique_id))
                    .body(BufferedBody::Complete { data: Some(Bytes::from(REQUEST_BODY)), trailers: None })
                    .unwrap();
                let request = CacheableHttpRequest::from_request(request);
                let upstream = MockUpstream;
                let cache_future =
                    CacheFuture::new(backend, request, upstream, req_pred, res_pred, ext, policy, DisabledOffload, NoopConcurrencyManager);
                std::hint::black_box(cache_future.await)
            }
        });
    });

    group.finish();
}

// ============================================================================
// Summary from Criterion Results
// ============================================================================

/// Read Criterion's benchmark results from JSON and print comparison table
fn print_comparison_summary(_c: &mut Criterion) {
    use std::fs;
    use std::path::{Path, PathBuf};

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║              STATIC vs DYNAMIC DISPATCH - CRITERION RESULTS                  ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!();

    // Criterion stores results in target/criterion/<group>/<benchmark>/new/estimates.json
    // Use CARGO_MANIFEST_DIR to find the workspace root (go up from hitbox-test to hitbox)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let base_path: PathBuf = PathBuf::from(manifest_dir)
        .parent() // hitbox workspace root
        .map(|p| p.join("target/criterion"))
        .unwrap_or_else(|| PathBuf::from("target/criterion"));

    // Helper to read mean time (in nanoseconds) from Criterion's estimates.json
    fn read_estimate(base: &Path, group: &str, bench: &str) -> Option<f64> {
        let path = base
            .join(group)
            .join(bench)
            .join("new")
            .join("estimates.json");
        let content = fs::read_to_string(&path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        // mean.point_estimate is in nanoseconds
        json.get("mean")?.get("point_estimate")?.as_f64()
    }

    fn format_ns(ns: f64) -> String {
        if ns >= 1000.0 {
            format!("{:.2} µs", ns / 1000.0)
        } else {
            format!("{:.0} ns", ns)
        }
    }

    fn calc_overhead(static_ns: f64, dynamic_ns: f64) -> String {
        let overhead = ((dynamic_ns - static_ns) / static_ns) * 100.0;
        if overhead >= 0.0 {
            format!("+{:.1}%", overhead)
        } else {
            format!("{:.1}%", overhead)
        }
    }

    // Backend cache operations
    let backend_comparisons = [
        ("compare_cache_read", "cache_read"),
        ("compare_cache_write", "cache_write"),
        ("compare_composition_read", "composition_read"),
        ("compare_composition_write", "composition_write"),
    ];

    // Header-only benchmarks
    let header_comparisons = [
        ("compare_request_predicates", "request_predicates"),
        ("compare_response_predicates", "response_predicates"),
        ("compare_extractors", "extractors"),
        ("compare_cache_future_hit", "cache_future_hit"),
        ("compare_cache_future_miss", "cache_future_miss"),
    ];

    // Body (jq) benchmarks
    let body_comparisons = [
        ("compare_body_request_predicates", "request_predicates"),
        ("compare_body_response_predicates", "response_predicates"),
        ("compare_body_extractors", "extractors"),
        ("compare_body_cache_future_hit", "cache_future_hit"),
        ("compare_body_cache_future_miss", "cache_future_miss"),
    ];

    println!("Backend operations (dyn Backend dispatch overhead):");
    println!("┌─────────────────────────┬────────────────┬────────────────┬────────────┐");
    println!("│ Benchmark               │ Static         │ Dynamic        │ Overhead   │");
    println!("├─────────────────────────┼────────────────┼────────────────┼────────────┤");

    for (group, label) in &backend_comparisons {
        let static_ns = read_estimate(&base_path, group, "static");
        let dynamic_ns = read_estimate(&base_path, group, "dynamic");

        match (static_ns, dynamic_ns) {
            (Some(s), Some(d)) => {
                println!(
                    "│ {:<23} │ {:>14} │ {:>14} │ {:>10} │",
                    label,
                    format_ns(s),
                    format_ns(d),
                    calc_overhead(s, d)
                );
            }
            _ => {
                println!(
                    "│ {:<23} │ {:>14} │ {:>14} │ {:>10} │",
                    label, "N/A", "N/A", "N/A"
                );
            }
        }
    }

    println!("└─────────────────────────┴────────────────┴────────────────┴────────────┘");
    println!();
    println!("Header-only predicates/extractors:");
    println!("┌─────────────────────────┬────────────────┬────────────────┬────────────┐");
    println!("│ Benchmark               │ Static         │ Dynamic        │ Overhead   │");
    println!("├─────────────────────────┼────────────────┼────────────────┼────────────┤");

    for (group, label) in &header_comparisons {
        let static_ns = read_estimate(&base_path, group, "static");
        let dynamic_ns = read_estimate(&base_path, group, "dynamic");

        match (static_ns, dynamic_ns) {
            (Some(s), Some(d)) => {
                println!(
                    "│ {:<23} │ {:>14} │ {:>14} │ {:>10} │",
                    label,
                    format_ns(s),
                    format_ns(d),
                    calc_overhead(s, d)
                );
            }
            _ => {
                println!(
                    "│ {:<23} │ {:>14} │ {:>14} │ {:>10} │",
                    label, "N/A", "N/A", "N/A"
                );
            }
        }
    }

    println!("└─────────────────────────┴────────────────┴────────────────┴────────────┘");
    println!();
    println!("Body (jq) predicates/extractors:");
    println!("┌─────────────────────────┬────────────────┬────────────────┬────────────┐");
    println!("│ Benchmark               │ Static         │ Dynamic        │ Overhead   │");
    println!("├─────────────────────────┼────────────────┼────────────────┼────────────┤");

    for (group, label) in &body_comparisons {
        let static_ns = read_estimate(&base_path, group, "static");
        let dynamic_ns = read_estimate(&base_path, group, "dynamic");

        match (static_ns, dynamic_ns) {
            (Some(s), Some(d)) => {
                println!(
                    "│ {:<23} │ {:>14} │ {:>14} │ {:>10} │",
                    label,
                    format_ns(s),
                    format_ns(d),
                    calc_overhead(s, d)
                );
            }
            _ => {
                println!(
                    "│ {:<23} │ {:>14} │ {:>14} │ {:>10} │",
                    label, "N/A", "N/A", "N/A"
                );
            }
        }
    }

    println!("└─────────────────────────┴────────────────┴────────────────┴────────────┘");
    println!();
    println!("Legend:");
    println!("  • Static:   Compile-time generics");
    println!("              - Backend: MokaBackend<...> (concrete type)");
    println!("              - Predicates/Extractors: impl Predicate, impl Extractor");
    println!("  • Dynamic:  Runtime trait objects (from YAML configuration)");
    println!("              - Backend: Arc<dyn Backend + Send>");
    println!("              - Predicates/Extractors: Arc<dyn Predicate>, Arc<dyn Extractor>");
    println!("  • Overhead: Percentage increase from static to dynamic dispatch");
    println!();
    println!("Note: Values are mean time per iteration from Criterion's statistical analysis");
    println!("      (100 samples). Results stored in target/criterion/");
    println!();
}

criterion_group!(
    benches,
    // Backend cache operations (static vs dynamic)
    bench_compare_cache_read,
    bench_compare_cache_write,
    // CompositionBackend operations (static vs dynamic)
    bench_compare_composition_read,
    bench_compare_composition_write,
    // Header-only comparison benchmarks (static vs dynamic)
    bench_compare_request_predicates,
    bench_compare_response_predicates,
    bench_compare_extractors,
    bench_compare_cache_future_hit,
    bench_compare_cache_future_miss,
    // Body (jq) comparison benchmarks (static vs dynamic)
    bench_compare_body_request_predicates,
    bench_compare_body_response_predicates,
    bench_compare_body_extractors,
    bench_compare_body_cache_future_hit,
    bench_compare_body_cache_future_miss,
    // Summary with real measurements
    print_comparison_summary
);
criterion_main!(benches);
