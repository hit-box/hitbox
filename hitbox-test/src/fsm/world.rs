use std::future::Ready;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::Utc;
use cucumber::World;
use hitbox::CacheContext;
use hitbox::concurrency::BroadcastConcurrencyManager;
use hitbox::fsm::CacheFuture;
use hitbox::policy::{ConcurrencyLimit, EnabledCacheConfig, PolicyConfig};
use hitbox_backend::composition::CompositionPolicy;
use hitbox_backend::composition::policy::RefillPolicy;
use hitbox_backend::{CacheBackend, CompositionBackend};
use hitbox_core::{
    CacheKey, CachePolicy, CacheValue, CacheablePolicyData, CacheableRequest, CacheableResponse,
    EntityPolicyConfig, Extractor, KeyPart, KeyParts, Offload, Predicate, PredicateResult,
    RequestCachePolicy, ResponseCachePolicy, SmolStr, Upstream,
};
use hitbox_moka::MokaBackend;

use crate::tracing::{SpanCollector, create_span_collector};

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Clone, Debug)]
pub struct SimpleRequest(pub u32);

impl CacheableRequest for SimpleRequest {
    type CachePolicyFuture<'a, P, E>
        = std::pin::Pin<Box<dyn std::future::Future<Output = RequestCachePolicy<Self>> + Send + 'a>>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a;

    fn cache_policy<'a, P, E>(
        self,
        predicates: P,
        extractors: E,
    ) -> Self::CachePolicyFuture<'a, P, E>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a,
    {
        Box::pin(async move {
            match predicates.check(self).await {
                PredicateResult::Cacheable(request) => {
                    let key_parts = extractors.get(request).await;
                    let (request, cache_key) = key_parts.into_cache_key();
                    RequestCachePolicy::Cacheable(CacheablePolicyData {
                        key: cache_key,
                        request,
                    })
                }
                PredicateResult::NonCacheable(request) => RequestCachePolicy::NonCacheable(request),
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SimpleResponse(pub u32);

impl CacheableResponse for SimpleResponse {
    type Cached = u32;
    type Subject = SimpleResponse;
    type IntoCachedFuture = Ready<CachePolicy<Self::Cached, Self>>;
    type FromCachedFuture = Ready<Self>;

    async fn cache_policy<P>(
        self,
        predicates: P,
        _config: &EntityPolicyConfig,
    ) -> ResponseCachePolicy<Self>
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(response) => {
                CachePolicy::Cacheable(CacheValue::new(response.0, None, None))
            }
            PredicateResult::NonCacheable(response) => CachePolicy::NonCacheable(response),
        }
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        std::future::ready(CachePolicy::Cacheable(self.0))
    }

    fn from_cached(cached: Self::Cached) -> Self::FromCachedFuture {
        std::future::ready(SimpleResponse(cached))
    }
}

// =============================================================================
// Configurable Predicates
// =============================================================================

#[derive(Debug, Clone, Copy)]
pub struct ConfigurableRequestPredicate {
    pub cacheable: bool,
}

#[async_trait::async_trait]
impl Predicate for ConfigurableRequestPredicate {
    type Subject = SimpleRequest;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        if self.cacheable {
            PredicateResult::Cacheable(subject)
        } else {
            PredicateResult::NonCacheable(subject)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigurableResponsePredicate {
    pub cacheable: bool,
}

#[async_trait::async_trait]
impl Predicate for ConfigurableResponsePredicate {
    type Subject = SimpleResponse;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        if self.cacheable {
            PredicateResult::Cacheable(subject)
        } else {
            PredicateResult::NonCacheable(subject)
        }
    }
}

// =============================================================================
// Extractor
// =============================================================================

#[derive(Debug)]
pub struct FixedKeyExtractor;

#[async_trait::async_trait]
impl Extractor for FixedKeyExtractor {
    type Subject = SimpleRequest;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let mut key_parts = KeyParts::new(subject);
        key_parts.push(KeyPart::new("fixed_key", Some("value")));
        key_parts
    }
}

// =============================================================================
// Configurable Upstream
// =============================================================================

pub struct ConfigurableUpstream {
    pub call_count: Arc<AtomicUsize>,
    pub delay_ms: u64,
}

impl ConfigurableUpstream {
    pub fn new(call_count: Arc<AtomicUsize>, delay_ms: u64) -> Self {
        Self {
            call_count,
            delay_ms,
        }
    }
}

impl Upstream<SimpleRequest> for ConfigurableUpstream {
    type Response = SimpleResponse;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = SimpleResponse> + Send>>;

    fn call(self, request: SimpleRequest) -> Self::Future {
        let call_count = self.call_count;
        let delay_ms = self.delay_ms;
        let response_value = request.0;
        Box::pin(async move {
            call_count.fetch_add(1, Ordering::SeqCst);
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            SimpleResponse(response_value)
        })
    }
}

// =============================================================================
// Test Offload (for composition backend)
// =============================================================================

#[derive(Clone, Debug)]
pub struct TestOffload;

impl Offload<'static> for TestOffload {
    #[allow(deprecated)]
    fn spawn<F>(&self, _kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future);
    }
}

// =============================================================================
// FSM Configuration
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct FsmConfig {
    pub cache_enabled: bool,
    pub request_cacheable: bool,
    pub response_cacheable: bool,
    pub concurrency: Option<ConcurrencyLimit>,
    pub ttl: Option<Duration>,
    pub stale: Option<Duration>,
}

// =============================================================================
// Composition Configuration
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct CompositionConfig {
    /// Whether to use CompositionBackend instead of single MokaBackend
    pub enabled: bool,
    /// Refill policy for composition backend
    pub refill_policy: RefillPolicy,
    /// Cache state for L1 (when composition is enabled)
    pub l1_state: CacheState,
    /// Cache state for L2 (when composition is enabled)
    pub l2_state: CacheState,
}

// =============================================================================
// Cache Pre-population State
// =============================================================================

#[derive(Debug, Clone, Default)]
pub enum CacheState {
    #[default]
    Empty,
    Fresh(u32),
    Stale(u32),
    Expired(u32),
}

// =============================================================================
// Test Results
// =============================================================================

#[derive(Debug, Default)]
pub struct TestResults {
    pub responses: Vec<(SimpleResponse, CacheContext)>,
    pub upstream_call_count: usize,
}

impl TestResults {
    pub fn upstream_call_count(&self) -> usize {
        self.upstream_call_count
    }

    pub fn all_responses_eq(&self, expected: u32) -> bool {
        self.responses.iter().all(|(r, _)| r.0 == expected)
    }
}

// =============================================================================
// FsmWorld - Cucumber World for FSM tests
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

static WORLD_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(World)]
#[world(init = Self::new)]
pub struct FsmWorld {
    pub id: u64,
    pub config: FsmConfig,
    pub cache_state: CacheState,
    pub upstream_delay_ms: u64,
    pub request_delay_ms: u64,
    pub results: TestResults,
    pub backend: MokaBackend,
    /// Composition backend configuration
    pub composition: CompositionConfig,
    /// L1 backend (for inspection when composition is enabled)
    pub l1_backend: MokaBackend,
    /// L2 backend (for inspection when composition is enabled)
    pub l2_backend: MokaBackend,
    /// Span collector for capturing FSM state transitions
    pub span_collector: SpanCollector,
}

impl FsmWorld {
    pub fn new() -> Self {
        let id = WORLD_ID_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
        Self {
            id,
            config: FsmConfig {
                cache_enabled: true,
                request_cacheable: true,
                response_cacheable: true,
                concurrency: ConcurrencyLimit::new(1),
                ttl: Some(Duration::from_secs(60)),
                stale: None,
            },
            cache_state: CacheState::Empty,
            upstream_delay_ms: 100,
            request_delay_ms: 10,
            results: TestResults::default(),
            backend: MokaBackend::builder().max_entries(100).build(),
            composition: CompositionConfig::default(),
            l1_backend: MokaBackend::builder().max_entries(100).build(),
            l2_backend: MokaBackend::builder().max_entries(100).build(),
            span_collector: create_span_collector(),
        }
    }

    pub async fn run_requests(&mut self, num_requests: usize, request_value: u32) {
        // Note: We no longer clear mock time at start because tests may run in parallel
        // and this would clear another test's mock time.
        // Instead, mock time is managed per-scenario in prepopulate_cache.

        let upstream_call_count = Arc::new(AtomicUsize::new(0));

        let request_pred = Arc::new(ConfigurableRequestPredicate {
            cacheable: self.config.request_cacheable,
        });
        let response_pred = Arc::new(ConfigurableResponsePredicate {
            cacheable: self.config.response_cacheable,
        });
        let extractor = Arc::new(FixedKeyExtractor);
        let concurrency_manager = Arc::new(BroadcastConcurrencyManager::<SimpleResponse>::new());

        let policy = Arc::new(if self.config.cache_enabled {
            PolicyConfig::Enabled(EnabledCacheConfig {
                ttl: self.config.ttl,
                stale: self.config.stale,
                concurrency: self.config.concurrency,
                policy: Default::default(),
            })
        } else {
            PolicyConfig::Disabled
        });

        // Pre-populate cache if needed
        if self.composition.enabled {
            self.prepopulate_composition_cache().await;
        } else {
            let backend = Arc::new(self.backend.clone());
            self.prepopulate_cache(&backend).await;
        }

        // Clear any previously captured spans
        self.span_collector.clear();

        let mut handles = Vec::new();

        for i in 0..num_requests {
            let request_pred = request_pred.clone();
            let response_pred = response_pred.clone();
            let extractor = extractor.clone();
            let policy = policy.clone();
            let concurrency_manager = concurrency_manager.clone();
            let upstream_call_count = upstream_call_count.clone();
            let upstream_delay_ms = self.upstream_delay_ms;
            let request_delay_ms = self.request_delay_ms;

            // Create appropriate backend based on composition configuration
            let composition_enabled = self.composition.enabled;
            let refill_policy = self.composition.refill_policy;
            let l1 = self.l1_backend.clone();
            let l2 = self.l2_backend.clone();
            let single_backend = self.backend.clone();

            // Clone the dispatch for use in spawned task
            let dispatch = self.span_collector.dispatch().clone();

            let handle = tokio::spawn(async move {
                if request_delay_ms > 0 && i > 0 {
                    tokio::time::sleep(Duration::from_millis(i as u64 * request_delay_ms)).await;
                }

                let upstream = ConfigurableUpstream::new(upstream_call_count, upstream_delay_ms);

                // Run the cache future with span capture enabled
                tracing::dispatcher::with_default(&dispatch, || {
                    // Use block_in_place to allow sync closure to await
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            if composition_enabled {
                                // Use CompositionBackend
                                let composition = CompositionBackend::new(l1, l2, TestOffload)
                                    .with_policy(CompositionPolicy::new().refill(refill_policy));
                                let backend = Arc::new(composition);

                                let cache_future = CacheFuture::new(
                                    backend,
                                    SimpleRequest(request_value),
                                    upstream,
                                    request_pred,
                                    response_pred,
                                    extractor,
                                    policy,
                                    hitbox_core::DisabledOffload, // No offload for tests
                                    concurrency_manager,
                                );

                                cache_future.await
                            } else {
                                // Use single MokaBackend
                                let backend = Arc::new(single_backend);

                                let cache_future = CacheFuture::new(
                                    backend,
                                    SimpleRequest(request_value),
                                    upstream,
                                    request_pred,
                                    response_pred,
                                    extractor,
                                    policy,
                                    hitbox_core::DisabledOffload, // No offload for tests
                                    concurrency_manager,
                                );

                                cache_future.await
                            }
                        })
                    })
                })
            });

            handles.push(handle);
        }

        let responses: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.expect("Task should not panic"))
            .collect();

        self.results = TestResults {
            responses,
            upstream_call_count: upstream_call_count.load(Ordering::SeqCst),
        };
    }

    async fn prepopulate_cache(&self, backend: &Arc<MokaBackend>) {
        let mut ctx = CacheContext::default().boxed();
        let cache_key = CacheKey::from_str("fixed_key", "value");

        match &self.cache_state {
            CacheState::Empty => {}
            CacheState::Fresh(value) => {
                // Fresh: expires in the future
                let expire = Some(Utc::now() + chrono::Duration::hours(1));
                let cache_value = CacheValue::new(*value, expire, None);
                let _ = backend
                    .set::<SimpleResponse>(&cache_key, &cache_value, &mut ctx)
                    .await;
            }
            CacheState::Stale(value) => {
                // Stale: expire is in the future, but stale threshold is in the past
                // This means: not expired yet, but past the "fresh" period
                let expire = Some(Utc::now() + chrono::Duration::hours(1));
                let stale = Some(Utc::now() - chrono::Duration::seconds(1));
                let cache_value = CacheValue::new(*value, expire, stale);
                let _ = backend
                    .set::<SimpleResponse>(&cache_key, &cache_value, &mut ctx)
                    .await;
            }
            CacheState::Expired(value) => {
                // Note: With moka backend, expired entries are immediately evicted due to TTL=0.
                // This means the FSM will see a cache miss (None), which also triggers upstream.
                // Both expired-entry and cache-miss paths lead to upstream being called, so
                // functionally this tests the same behavior.
                let expire = Some(Utc::now() - chrono::Duration::hours(1));
                let stale = Some(Utc::now() - chrono::Duration::minutes(30));
                let cache_value = CacheValue::new(*value, expire, stale);
                let _ = backend
                    .set::<SimpleResponse>(&cache_key, &cache_value, &mut ctx)
                    .await;
            }
        }
    }

    /// Pre-populate L1 and L2 caches based on composition configuration.
    async fn prepopulate_composition_cache(&self) {
        let cache_key = CacheKey::from_str("fixed_key", "value");

        // Populate L1 based on l1_state
        self.populate_backend(&self.l1_backend, &cache_key, &self.composition.l1_state)
            .await;

        // Populate L2 based on l2_state
        self.populate_backend(&self.l2_backend, &cache_key, &self.composition.l2_state)
            .await;
    }

    /// Helper to populate a single backend with given cache state.
    async fn populate_backend(
        &self,
        backend: &MokaBackend,
        cache_key: &CacheKey,
        state: &CacheState,
    ) {
        let mut ctx = CacheContext::default().boxed();
        match state {
            CacheState::Empty => {}
            CacheState::Fresh(value) => {
                let expire = Some(Utc::now() + chrono::Duration::hours(1));
                let cache_value = CacheValue::new(*value, expire, None);
                let _ = backend
                    .set::<SimpleResponse>(cache_key, &cache_value, &mut ctx)
                    .await;
            }
            CacheState::Stale(value) => {
                let expire = Some(Utc::now() + chrono::Duration::hours(1));
                let stale = Some(Utc::now() - chrono::Duration::seconds(1));
                let cache_value = CacheValue::new(*value, expire, stale);
                let _ = backend
                    .set::<SimpleResponse>(cache_key, &cache_value, &mut ctx)
                    .await;
            }
            CacheState::Expired(value) => {
                // Note: With moka backend, expired entries are immediately evicted.
                // See comment in prepopulate_cache for details.
                let expire = Some(Utc::now() - chrono::Duration::hours(1));
                let stale = Some(Utc::now() - chrono::Duration::minutes(30));
                let cache_value = CacheValue::new(*value, expire, stale);
                let _ = backend
                    .set::<SimpleResponse>(cache_key, &cache_value, &mut ctx)
                    .await;
            }
        }
    }

    pub async fn cache_contains_value(&self, expected: u32) -> bool {
        let cache_key = CacheKey::from_str("fixed_key", "value");
        let mut ctx = CacheContext::default().boxed();
        let result = self
            .backend
            .get::<SimpleResponse>(&cache_key, &mut ctx)
            .await;
        if let Ok(Some(cached)) = result {
            let inner = cached.into_inner();
            inner == expected
        } else {
            false
        }
    }

    pub async fn cache_is_empty(&self) -> bool {
        let cache_key = CacheKey::from_str("fixed_key", "value");
        let mut ctx = CacheContext::default().boxed();
        self.backend
            .get::<SimpleResponse>(&cache_key, &mut ctx)
            .await
            .ok()
            .flatten()
            .is_none()
    }

    // =========================================================================
    // Composition L1/L2 inspection methods
    // =========================================================================

    /// Check if L1 contains a specific value.
    pub async fn l1_contains_value(&self, expected: u32) -> bool {
        let cache_key = CacheKey::from_str("fixed_key", "value");
        let mut ctx = CacheContext::default().boxed();
        if let Ok(Some(cached)) = self
            .l1_backend
            .get::<SimpleResponse>(&cache_key, &mut ctx)
            .await
        {
            cached.into_inner() == expected
        } else {
            false
        }
    }

    /// Check if L1 cache is empty.
    pub async fn l1_is_empty(&self) -> bool {
        let cache_key = CacheKey::from_str("fixed_key", "value");
        let mut ctx = CacheContext::default().boxed();
        self.l1_backend
            .get::<SimpleResponse>(&cache_key, &mut ctx)
            .await
            .ok()
            .flatten()
            .is_none()
    }

    /// Check if L2 contains a specific value.
    pub async fn l2_contains_value(&self, expected: u32) -> bool {
        let cache_key = CacheKey::from_str("fixed_key", "value");
        let mut ctx = CacheContext::default().boxed();
        if let Ok(Some(cached)) = self
            .l2_backend
            .get::<SimpleResponse>(&cache_key, &mut ctx)
            .await
        {
            cached.into_inner() == expected
        } else {
            false
        }
    }

    /// Check if L2 cache is empty.
    pub async fn l2_is_empty(&self) -> bool {
        let cache_key = CacheKey::from_str("fixed_key", "value");
        let mut ctx = CacheContext::default().boxed();
        self.l2_backend
            .get::<SimpleResponse>(&cache_key, &mut ctx)
            .await
            .ok()
            .flatten()
            .is_none()
    }
}

impl Default for FsmWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FsmWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FsmWorld")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("cache_state", &self.cache_state)
            .field("upstream_delay_ms", &self.upstream_delay_ms)
            .field("request_delay_ms", &self.request_delay_ms)
            .field("results", &self.results)
            .field("composition", &self.composition)
            .finish_non_exhaustive()
    }
}
