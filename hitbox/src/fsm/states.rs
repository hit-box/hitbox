//! FSM state types and resolved state structs.
//!
//! Each state struct represents resolved async data and has a `.transition()` method
//! that returns the appropriate transition enum. The transition enum then has
//! `.into_state()` to convert to the outer `State` enum.
//!
//! Flow: poll future → create state struct → `.transition()` → `.into_state()`

use std::fmt::{self, Debug};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use futures::future::BoxFuture;
use futures::ready;
use hitbox_backend::BackendError;
use hitbox_core::{
    BoxContext, CachePolicy, CacheValue, Cacheable, CacheablePolicyData, EntityPolicyConfig,
    Predicate, ReadMode, RequestCachePolicy, ResponseCachePolicy, Upstream,
};
use pin_project::pin_project;
use tokio::sync::OwnedSemaphorePermit;
use tracing::{Instrument, Level, Span, debug, field, instrument::Instrumented, span, warn};

use crate::backend::CacheBackend;
use crate::concurrency::{ConcurrencyDecision, ConcurrencyError, ConcurrencyManager};
use crate::fsm::transitions::{
    AwaitResponseTransition, CheckRequestCachePolicyTransition, CheckResponseCachePolicyTransition,
    ConvertResponseTransition, HandleStaleTransition, InitialTransition, PollCacheTransition,
    PollUpstreamTransition, UpdateCacheTransition,
};
use crate::policy::{EnabledCacheConfig, PolicyConfig, StalePolicy};
use crate::{CacheKey, CacheState, CacheStatus, CacheableRequest, CacheableResponse, Extractor};

// =============================================================================
// Helper Types
// =============================================================================

/// Wrapper for `Option<&CacheKey>` that implements `Display` without allocation.
struct OptionalKey<'a>(Option<&'a CacheKey>);

impl fmt::Display for OptionalKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(key) => fmt::Display::fmt(key, f),
            None => Ok(()),
        }
    }
}

// =============================================================================
// Type Aliases
// =============================================================================

pub type CacheResult<T> = Result<Option<CacheValue<T>>, BackendError>;
/// Future that polls the cache and returns (result, context)
pub type PollCacheFuture<T> = BoxFuture<'static, (CacheResult<T>, BoxContext)>;
/// Future that updates the cache and returns (backend_result, response, context)
pub type UpdateCacheFuture<T> = BoxFuture<'static, (Result<(), BackendError>, T, BoxContext)>;
pub type AwaitResponseFuture<T> = BoxFuture<'static, Result<T, ConcurrencyError>>;
/// Future that checks request cache policy
pub type RequestCachePolicyFuture<'a, T> = BoxFuture<'a, RequestCachePolicy<T>>;

// =============================================================================
// ConvertResponseFuture - Zero-cost wrapper using GAT
// =============================================================================

/// Future that converts cached value to response using the GAT `FromCachedFuture`.
///
/// This wrapper avoids boxing by directly using the response type's `FromCachedFuture`.
/// For types where `FromCachedFuture = Ready<Self>` (like `CacheableHttpResponse`),
/// this provides zero-cost cache hits with no allocation.
#[pin_project]
pub struct ConvertResponseFuture<Res: CacheableResponse> {
    #[pin]
    inner: Res::FromCachedFuture,
    ctx: Option<BoxContext>,
}

impl<Res: CacheableResponse> ConvertResponseFuture<Res> {
    /// Create a new ConvertResponseFuture from a cached value and context.
    pub fn new(cached: Res::Cached, ctx: BoxContext) -> Self {
        Self {
            inner: Res::from_cached(cached),
            ctx: Some(ctx),
        }
    }
}

impl<Res: CacheableResponse> std::future::Future for ConvertResponseFuture<Res> {
    type Output = (Res, BoxContext);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let response = ready!(this.inner.poll(cx));
        Poll::Ready((response, this.ctx.take().expect("polled after completion")))
    }
}

// =============================================================================
// State Enum
// =============================================================================

#[allow(missing_docs)]
#[pin_project(project = StateProj)]
pub enum State<'req, Res, Req, U, ReqP, E>
where
    Res: CacheableResponse,
    Req: CacheableRequest + 'req,
    U: Upstream<Req, Response = Res> + 'req,
    ReqP: Predicate<Subject = Req>,
    E: Extractor<Subject = Req>,
{
    /// Initial state - all data needed for the first transition
    Initial(Option<Initial<Req, ReqP, E, U>>),
    /// Checking if request should be cached
    CheckRequestCachePolicy {
        #[pin]
        cache_policy_future: RequestCachePolicyFuture<'req, Req>,
        state: Option<CheckRequestCachePolicy<U>>,
    },
    /// Polling the cache backend - context is captured in the future
    PollCache {
        #[pin]
        poll_cache: PollCacheFuture<Res::Cached>,
        state: Option<PollCache<Req, U>>,
    },
    /// Converting cached value to response (cache hit, no refill needed)
    ConvertResponse {
        #[pin]
        response_future: ConvertResponseFuture<Res>,
        state: Option<ConvertResponse>,
    },
    /// Handling stale cache hit - convert to response then apply stale policy
    HandleStale {
        #[pin]
        response_future: ConvertResponseFuture<Res>,
        state: Option<HandleStale<Req, U>>,
    },
    /// Awaiting response from another concurrent request
    AwaitResponse {
        #[pin]
        await_response_future: AwaitResponseFuture<Res>,
        state: Option<AwaitResponse<Req, U>>,
    },
    /// Polling upstream service
    PollUpstream {
        #[pin]
        upstream_future: Instrumented<U::Future<'req>>,
        state: Option<PollUpstream>,
    },
    /// Checking if response should be cached
    CheckResponseCachePolicy {
        #[pin]
        cache_policy: BoxFuture<'static, ResponseCachePolicy<Res>>,
        state: Option<CheckResponseCachePolicy>,
    },
    /// Updating cache with response - context is captured in the future
    UpdateCache {
        #[pin]
        update_cache_future: UpdateCacheFuture<Res>,
        state: Option<UpdateCache>,
    },
    /// Final state with response
    Response(Option<Response<Res>>),
}

impl<Res, Req, U, ReqP, E> Debug for State<'_, Res, Req, U, ReqP, E>
where
    Res: CacheableResponse,
    Req: CacheableRequest,
    U: Upstream<Req, Response = Res>,
    ReqP: Predicate<Subject = Req>,
    E: Extractor<Subject = Req>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Initial(_) => f.write_str("State::Initial"),
            State::CheckRequestCachePolicy { .. } => f.write_str("State::CheckRequestCachePolicy"),
            State::PollCache { .. } => f.write_str("State::PollCache"),
            State::ConvertResponse { .. } => f.write_str("State::ConvertResponse"),
            State::HandleStale { .. } => f.write_str("State::HandleStale"),
            State::AwaitResponse { .. } => f.write_str("State::AwaitResponse"),
            State::CheckResponseCachePolicy { .. } => {
                f.write_str("State::CheckResponseCachePolicy")
            }
            State::PollUpstream { .. } => f.write_str("State::PollUpstream"),
            State::UpdateCache { .. } => f.write_str("State::UpdateCache"),
            State::Response(_) => f.write_str("State::Response"),
        }
    }
}

// =============================================================================
// Initial
// =============================================================================

/// Data gathered from Initial state (synchronous).
pub struct Initial<Req, ReqP, E, U> {
    pub request: Req,
    pub predicates: ReqP,
    pub extractors: E,
    pub ctx: BoxContext,
    pub upstream: U,
    /// Tracing span for this state.
    pub span: Span,
}

impl<Req, ReqP, E, U> Initial<Req, ReqP, E, U> {
    /// Create a new Initial state with its tracing span.
    pub fn new(
        request: Req,
        predicates: ReqP,
        extractors: E,
        ctx: BoxContext,
        upstream: U,
        parent: &Span,
    ) -> Self {
        Self {
            request,
            predicates,
            extractors,
            ctx,
            upstream,
            span: span!(parent: parent, Level::TRACE, "fsm.Initial"),
        }
    }
}

impl<Req, ReqP, E, U> Initial<Req, ReqP, E, U>
where
    Req: CacheableRequest + Send,
    ReqP: Predicate<Subject = Req> + Send + Sync,
    E: Extractor<Subject = Req> + Send + Sync,
    U: Upstream<Req>,
{
    /// Transition from Initial state.
    ///
    /// Based on policy configuration:
    /// - Enabled: create CheckRequestCachePolicy future
    /// - Disabled: call upstream directly
    pub fn transition<'req>(mut self, policy: &PolicyConfig) -> InitialTransition<'req, Req, U>
    where
        Req: 'req,
        ReqP: 'req,
        E: 'req,
        U: 'req,
    {
        match policy {
            PolicyConfig::Enabled(_) => {
                // Box the RPITIT future for storage in FSM state
                let cache_policy_future =
                    Box::pin(self.request.cache_policy(self.predicates, self.extractors));
                InitialTransition::CheckRequestCachePolicy {
                    cache_policy_future,
                    ctx: self.ctx,
                    upstream: self.upstream,
                }
            }
            PolicyConfig::Disabled => {
                let upstream_future = self.upstream.call(self.request);
                InitialTransition::PollUpstream {
                    upstream_future,
                    ctx: self.ctx,
                }
            }
        }
    }
}

impl<Req, ReqP, E, U> std::fmt::Debug for Initial<Req, ReqP, E, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Initial").finish_non_exhaustive()
    }
}

// =============================================================================
// Response
// =============================================================================

/// Terminal state with final response.
pub struct Response<Res> {
    pub response: Res,
    pub ctx: BoxContext,
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl<Res> Response<Res> {
    /// Create a new Response state with its tracing span.
    ///
    /// The `status` and `source` fields will be recorded when the response is finalized.
    pub fn new(response: Res, ctx: BoxContext, parent: &Span) -> Self {
        Self {
            response,
            ctx,
            span: span!(
                parent: parent,
                Level::TRACE,
                "fsm.Response",
                cache.status = field::Empty,
                cache.source = field::Empty
            ),
        }
    }
}

impl<Res> std::fmt::Debug for Response<Res> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response").finish_non_exhaustive()
    }
}

// =============================================================================
// PollUpstream
// =============================================================================

/// Data for PollUpstream state (non-pinned part).
///
/// The upstream future is stored separately in the State enum to allow pinning.
/// When the future completes, this data is taken and passed to transition().
pub struct PollUpstream {
    pub permit: Option<OwnedSemaphorePermit>,
    pub ctx: BoxContext,
    pub cache_key: Option<CacheKey>,
    /// Start time for measuring upstream call duration.
    pub upstream_start: Instant,
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl PollUpstream {
    /// Create a new PollUpstream state with its tracing span, instrumenting the provided future.
    ///
    /// Returns both the state and the instrumented future, since the future needs to be
    /// instrumented with the same span that's stored in the state.
    pub fn with_future<F: Sized>(
        permit: Option<OwnedSemaphorePermit>,
        ctx: BoxContext,
        cache_key: Option<CacheKey>,
        future: F,
        parent: &Span,
    ) -> (Self, Instrumented<F>) {
        let has_permit = permit.is_some();
        let span = span!(
            parent: parent,
            Level::TRACE,
            "fsm.PollUpstream",
            cache.key = %OptionalKey(cache_key.as_ref()),
            concurrency.permit = has_permit
        );
        (
            Self {
                permit,
                ctx,
                cache_key,
                upstream_start: Instant::now(),
                span: span.clone(),
            },
            future.instrument(span),
        )
    }

    /// Transition from PollUpstream state after future completes.
    ///
    /// This merges the old PollUpstream → UpstreamPolled → next state transitions
    /// into a single step, since UpstreamPolled was a synchronous state.
    pub fn transition<Res, ResP>(
        self,
        upstream_result: Res,
        predicates: ResP,
        policy: &PolicyConfig,
    ) -> PollUpstreamTransition<Res>
    where
        Res: CacheableResponse + Send + 'static,
        ResP: Predicate<Subject = Res::Subject> + Send + Sync + 'static,
    {
        // Record upstream duration metric
        crate::metrics::record_upstream_duration(self.upstream_start.elapsed());

        match self.cache_key {
            Some(cache_key) => {
                let entity_config = match policy {
                    PolicyConfig::Enabled(config) => EntityPolicyConfig {
                        ttl: config.ttl,
                        stale_ttl: config.stale,
                    },
                    PolicyConfig::Disabled => EntityPolicyConfig::default(),
                };
                PollUpstreamTransition::CheckResponseCachePolicy {
                    cache_policy_future: Box::pin(async move {
                        upstream_result
                            .cache_policy(predicates, &entity_config)
                            .await
                    }),
                    permit: self.permit,
                    ctx: self.ctx,
                    cache_key,
                }
            }
            None => PollUpstreamTransition::Response(Response {
                response: upstream_result,
                ctx: self.ctx,
                span: Span::none(),
            }),
        }
    }
}

impl std::fmt::Debug for PollUpstream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollUpstream")
            .field("has_permit", &self.permit.is_some())
            .field("cache_key", &self.cache_key)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// CheckResponseCachePolicy
// =============================================================================

/// Data for CheckResponseCachePolicy state (non-pinned part).
///
/// The cache policy future is stored separately in the State enum to allow pinning.
/// When the future completes, this data is taken and passed to transition().
pub struct CheckResponseCachePolicy {
    pub permit: Option<OwnedSemaphorePermit>,
    pub ctx: BoxContext,
    pub cache_key: CacheKey,
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl CheckResponseCachePolicy {
    /// Create a new CheckResponseCachePolicy state with its tracing span.
    ///
    /// The `cacheable` field will be recorded after the policy check completes.
    pub fn new(
        permit: Option<OwnedSemaphorePermit>,
        ctx: BoxContext,
        cache_key: CacheKey,
        parent: &Span,
    ) -> Self {
        let has_permit = permit.is_some();
        Self {
            permit,
            ctx,
            cache_key: cache_key.clone(),
            span: span!(
                parent: parent,
                Level::TRACE,
                "fsm.CheckResponseCachePolicy",
                cache.key = %cache_key,
                concurrency.permit = has_permit,
                cache.cacheable = field::Empty
            ),
        }
    }

    /// Transition from CheckResponseCachePolicy state after future completes.
    pub fn transition<Res, B, C>(
        self,
        policy: CachePolicy<CacheValue<Res::Cached>, Res>,
        backend: Arc<B>,
        concurrency_manager: &C,
    ) -> CheckResponseCachePolicyTransition<Res>
    where
        Res: CacheableResponse + Send + 'static,
        Res::Cached: Cacheable + Send,
        B: CacheBackend + Send + Sync + 'static,
        C: ConcurrencyManager<Res>,
    {
        // Record cacheable decision to span
        self.span.record(
            "cache.cacheable",
            matches!(&policy, CachePolicy::Cacheable(_)),
        );

        match policy {
            CachePolicy::Cacheable(cache_value) => {
                if self.permit.is_some() {
                    concurrency_manager.resolve(&self.cache_key, &cache_value);
                }
                let cache_key = self.cache_key;
                let mut ctx = self.ctx;
                let update_cache_future = Box::pin(async move {
                    let update_cache_result =
                        backend.set::<Res>(&cache_key, &cache_value, &mut ctx).await;
                    let upstream_result = Res::from_cached(cache_value.into_inner()).await;
                    (update_cache_result, upstream_result, ctx)
                });
                CheckResponseCachePolicyTransition::UpdateCache {
                    update_cache_future,
                }
            }
            CachePolicy::NonCacheable(response) => {
                if self.permit.is_some() {
                    concurrency_manager.cleanup(&self.cache_key);
                }
                CheckResponseCachePolicyTransition::Response(Response {
                    response,
                    ctx: self.ctx,
                    span: Span::none(),
                })
            }
        }
    }
}

impl std::fmt::Debug for CheckResponseCachePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckResponseCachePolicy")
            .field("has_permit", &self.permit.is_some())
            .field("cache_key", &self.cache_key)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// CheckRequestCachePolicy
// =============================================================================

/// Data for CheckRequestCachePolicy state (non-pinned part).
///
/// The cache policy future is stored separately in the State enum to allow pinning.
/// When the future completes, this data is taken and passed to transition().
pub struct CheckRequestCachePolicy<U> {
    pub ctx: BoxContext,
    pub upstream: U,
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl<U> CheckRequestCachePolicy<U> {
    /// Create a new CheckRequestCachePolicy state with its tracing span.
    ///
    /// The `cacheable` field will be recorded after the policy check completes.
    pub fn new(ctx: BoxContext, upstream: U, parent: &Span) -> Self {
        Self {
            ctx,
            upstream,
            span: span!(parent: parent, Level::TRACE, "fsm.CheckRequestCachePolicy", cache.cacheable = field::Empty),
        }
    }

    /// Transition from CheckRequestCachePolicy state after future completes.
    pub fn transition<'req, Req, Res, B>(
        mut self,
        policy: RequestCachePolicy<Req>,
        backend: Arc<B>,
        cache_key_storage: &mut Option<CacheKey>,
    ) -> CheckRequestCachePolicyTransition<'req, Req, Res, U>
    where
        Req: CacheableRequest + 'req,
        Res: CacheableResponse,
        Res::Cached: Cacheable + Send,
        U: Upstream<Req, Response = Res> + 'req,
        B: CacheBackend + Send + Sync + 'static,
    {
        // Record cacheable decision to span
        self.span.record(
            "cache.cacheable",
            matches!(&policy, CachePolicy::Cacheable(_)),
        );
        match policy {
            CachePolicy::Cacheable(CacheablePolicyData { key, request }) => {
                let cache_key_for_get = key.clone();
                debug!(?key, "FSM looking up cache key");
                let _ = cache_key_storage.insert(key.clone());
                let mut ctx = self.ctx;
                let poll_cache = Box::pin(async move {
                    let result = backend.get::<Res>(&cache_key_for_get, &mut ctx).await;
                    debug!(
                        found = result.as_ref().map(|r| r.is_some()).unwrap_or(false),
                        "FSM cache lookup result"
                    );
                    (result, ctx)
                });
                CheckRequestCachePolicyTransition::PollCache {
                    poll_cache,
                    request,
                    cache_key: key,
                    upstream: self.upstream,
                }
            }
            CachePolicy::NonCacheable(request) => {
                let upstream_future = self.upstream.call(request);
                CheckRequestCachePolicyTransition::PollUpstream {
                    upstream_future,
                    ctx: self.ctx,
                }
            }
        }
    }
}

impl<U> std::fmt::Debug for CheckRequestCachePolicy<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckRequestCachePolicy")
            .finish_non_exhaustive()
    }
}

// =============================================================================
// PollCache
// =============================================================================

/// Data for PollCache state (non-pinned part).
///
/// The poll cache future is stored separately in the State enum to allow pinning.
/// When the future completes, this data is taken and passed to transition().
pub struct PollCache<Req, U> {
    pub request: Req,
    pub cache_key: CacheKey,
    pub upstream: U,
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl<Req, U> PollCache<Req, U> {
    /// Create a new PollCache state with its tracing span.
    pub fn new(request: Req, cache_key: CacheKey, upstream: U, parent: &Span) -> Self {
        Self {
            request,
            cache_key: cache_key.clone(),
            upstream,
            span: span!(parent: parent, Level::TRACE, "fsm.PollCache", cache.key = %cache_key, concurrency.decision = field::Empty),
        }
    }

    /// Transition from PollCache state after future completes.
    ///
    /// On cache miss or expired, checks concurrency policy and transitions directly
    /// to either `ConcurrentPollUpstream` or `PollUpstream`.
    pub fn transition<'req, Res, B, C>(
        self,
        cache_result: CacheResult<Res::Cached>,
        mut ctx: BoxContext,
        backend: Arc<B>,
        policy: &PolicyConfig,
        concurrency_manager: &C,
    ) -> PollCacheTransition<'req, Res, Req, U>
    where
        Res: CacheableResponse + Send + 'static,
        Res::Cached: Cacheable + Send,
        B: CacheBackend + Send + Sync + 'static,
        U: Upstream<Req, Response = Res> + 'req,
        Req: 'req,
        C: ConcurrencyManager<Res>,
    {
        let cached = cache_result
            .inspect_err(|err| warn!("Cache error: {err:?}"))
            .unwrap_or_default();

        match cached {
            Some(cached_value) => {
                let cache_state = cached_value.cache_state();
                ctx.set_status(CacheStatus::Hit);

                match cache_state {
                    CacheState::Actual(value) => {
                        if ctx.read_mode() == ReadMode::Refill {
                            let cache_key = self.cache_key;
                            let update_cache_future = Box::pin(async move {
                                let update_result =
                                    backend.set::<Res>(&cache_key, &value, &mut ctx).await;
                                let response = Res::from_cached(value.into_inner()).await;
                                (update_result, response, ctx)
                            });
                            PollCacheTransition::UpdateCache {
                                update_cache_future,
                            }
                        } else {
                            let cache_key = self.cache_key;
                            // Zero-cost conversion using GAT - no boxing!
                            let response_future =
                                ConvertResponseFuture::new(value.into_inner(), ctx);
                            PollCacheTransition::ConvertResponse {
                                response_future,
                                cache_key,
                            }
                        }
                    }
                    CacheState::Stale(value) => {
                        let cache_key = self.cache_key;
                        let request = self.request;
                        let upstream = self.upstream;
                        // Zero-cost conversion using GAT - no boxing!
                        let response_future = ConvertResponseFuture::new(value.into_inner(), ctx);
                        PollCacheTransition::HandleStale {
                            response_future,
                            request,
                            cache_key,
                            upstream,
                        }
                    }
                    CacheState::Expired(_value) => {
                        ctx.set_status(CacheStatus::Miss);
                        self.transition_to_upstream(ctx, policy, concurrency_manager)
                    }
                }
            }
            None => self.transition_to_upstream(ctx, policy, concurrency_manager),
        }
    }

    /// Helper to transition to upstream based on concurrency policy.
    fn transition_to_upstream<'req, Res, C>(
        mut self,
        ctx: BoxContext,
        policy: &PolicyConfig,
        concurrency_manager: &C,
    ) -> PollCacheTransition<'req, Res, Req, U>
    where
        Res: CacheableResponse,
        U: Upstream<Req, Response = Res> + 'req,
        Req: 'req,
        C: ConcurrencyManager<Res>,
    {
        match policy {
            PolicyConfig::Enabled(EnabledCacheConfig {
                concurrency: Some(concurrency),
                ..
            }) => match concurrency_manager.check(&self.cache_key, *concurrency) {
                ConcurrencyDecision::Proceed(permit) => {
                    self.span.record("concurrency.decision", "proceed");
                    let upstream_future = self.upstream.call(self.request);
                    PollCacheTransition::PollUpstream {
                        upstream_future,
                        permit: Some(permit),
                        ctx,
                        cache_key: self.cache_key,
                    }
                }
                ConcurrencyDecision::ProceedWithoutPermit => {
                    self.span
                        .record("concurrency.decision", "proceed_without_permit");
                    let upstream_future = self.upstream.call(self.request);
                    PollCacheTransition::PollUpstream {
                        upstream_future,
                        permit: None,
                        ctx,
                        cache_key: self.cache_key,
                    }
                }
                ConcurrencyDecision::Await(await_future) => {
                    self.span.record("concurrency.decision", "await");
                    PollCacheTransition::AwaitResponse {
                        await_response_future: await_future,
                        request: self.request,
                        ctx,
                        cache_key: self.cache_key,
                        upstream: self.upstream,
                    }
                }
            },
            _ => {
                self.span.record("concurrency.decision", "disabled");
                let upstream_future = self.upstream.call(self.request);
                PollCacheTransition::PollUpstream {
                    upstream_future,
                    permit: None,
                    ctx,
                    cache_key: self.cache_key,
                }
            }
        }
    }
}

impl<Req, U> std::fmt::Debug for PollCache<Req, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollCache")
            .field("cache_key", &self.cache_key)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// ConvertResponse
// =============================================================================

/// Data for ConvertResponse state (non-pinned part).
///
/// The response future is stored separately in the State enum to allow pinning.
/// When the future completes, this data is taken and passed to transition().
pub struct ConvertResponse {
    /// Cache key for logging/tracing purposes.
    pub cache_key: CacheKey,
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl ConvertResponse {
    /// Create a new ConvertResponse state with its tracing span.
    pub fn new(cache_key: CacheKey, parent: &Span) -> Self {
        Self {
            cache_key: cache_key.clone(),
            span: span!(parent: parent, Level::TRACE, "fsm.ConvertResponse", cache.key = %cache_key),
        }
    }

    /// Transition from ConvertResponse state after future completes.
    pub fn transition<Res>(self, response: Res, ctx: BoxContext) -> ConvertResponseTransition<Res> {
        debug!(cache.key = %self.cache_key, "ConvertResponse transition");
        ConvertResponseTransition::Response(Response {
            response,
            ctx,
            span: Span::none(),
        })
    }
}

impl std::fmt::Debug for ConvertResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConvertResponse")
            .field("cache_key", &self.cache_key)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// HandleStale
// =============================================================================

/// Data for HandleStale state (non-pinned part).
///
/// The response future is stored separately in the State enum to allow pinning.
/// When the future completes, this data is taken and passed to transition().
pub struct HandleStale<Req, U> {
    pub request: Req,
    pub cache_key: CacheKey,
    pub upstream: U,
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl<Req, U> HandleStale<Req, U> {
    /// Create a new HandleStale state with its tracing span.
    pub fn new(request: Req, cache_key: CacheKey, upstream: U, parent: &Span) -> Self {
        Self {
            request,
            cache_key: cache_key.clone(),
            upstream,
            span: span!(parent: parent, Level::TRACE, "fsm.HandleStale", cache.key = %cache_key, stale.policy = field::Empty),
        }
    }
}

/// Data needed for background offload revalidation.
pub struct OffloadData<Req, U> {
    pub request: Req,
    pub cache_key: CacheKey,
    pub upstream: U,
}

/// Result of HandleStale transition, including optional offload data.
pub struct HandleStaleResult<'req, Res, Req, U>
where
    U: Upstream<Req, Response = Res> + 'req,
    Req: 'req,
{
    pub transition: HandleStaleTransition<'req, Res, Req, U>,
    pub offload_data: Option<OffloadData<Req, U>>,
}

impl<Req, U> HandleStale<Req, U> {
    /// Transition from HandleStale state after future completes.
    ///
    /// Returns a result containing the transition and optional offload data.
    /// For `StalePolicy::OffloadRevalidate`, the caller is responsible for spawning
    /// the background revalidation using the returned `offload_data`.
    pub fn transition<'req, Res>(
        mut self,
        response: Res,
        mut ctx: BoxContext,
        policy: &PolicyConfig,
    ) -> HandleStaleResult<'req, Res, Req, U>
    where
        Res: CacheableResponse,
        U: Upstream<Req, Response = Res> + 'req,
        Req: 'req,
    {
        let stale_policy = match policy {
            PolicyConfig::Enabled(EnabledCacheConfig { policy, .. }) => policy.stale,
            PolicyConfig::Disabled => StalePolicy::Return,
        };

        match stale_policy {
            StalePolicy::Return => {
                self.span.record("stale.policy", "return");
                ctx.set_status(CacheStatus::Stale);
                HandleStaleResult {
                    transition: HandleStaleTransition::Response(Response {
                        response,
                        ctx,
                        span: Span::none(),
                    }),
                    offload_data: None,
                }
            }
            StalePolicy::Revalidate => {
                self.span.record("stale.policy", "revalidate");
                ctx.set_status(CacheStatus::Miss);
                let upstream_future = self.upstream.call(self.request);
                HandleStaleResult {
                    transition: HandleStaleTransition::Revalidate {
                        upstream_future,
                        ctx,
                        cache_key: self.cache_key,
                    },
                    offload_data: None,
                }
            }
            StalePolicy::OffloadRevalidate => {
                self.span.record("stale.policy", "offload");
                ctx.set_status(CacheStatus::Stale);
                HandleStaleResult {
                    transition: HandleStaleTransition::Response(Response {
                        response,
                        ctx,
                        span: Span::none(),
                    }),
                    offload_data: Some(OffloadData {
                        request: self.request,
                        cache_key: self.cache_key,
                        upstream: self.upstream,
                    }),
                }
            }
        }
    }
}

impl<Req, U> std::fmt::Debug for HandleStale<Req, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandleStale")
            .field("cache_key", &self.cache_key)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// AwaitResponse
// =============================================================================

/// Data for AwaitResponse state (non-pinned part).
///
/// The await response future is stored separately in the State enum to allow pinning.
/// When the future completes, this data is taken and passed to transition().
pub struct AwaitResponse<Req, U> {
    pub request: Req,
    pub ctx: BoxContext,
    pub cache_key: CacheKey,
    pub upstream: U,
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl<Req, U> AwaitResponse<Req, U> {
    /// Create a new AwaitResponse state with its tracing span.
    pub fn new(
        request: Req,
        ctx: BoxContext,
        cache_key: CacheKey,
        upstream: U,
        parent: &Span,
    ) -> Self {
        Self {
            request,
            ctx,
            cache_key: cache_key.clone(),
            upstream,
            span: span!(parent: parent, Level::TRACE, "fsm.AwaitResponse", cache.key = %cache_key),
        }
    }

    /// Transition from AwaitResponse state after future completes.
    ///
    /// On success, returns the response directly.
    /// On concurrency error, falls back to calling upstream.
    pub fn transition<'req, Res, C>(
        mut self,
        result: Result<Res, ConcurrencyError>,
        concurrency_manager: &C,
    ) -> AwaitResponseTransition<'req, Res, Req, U>
    where
        Res: CacheableResponse,
        U: Upstream<Req, Response = Res> + 'req,
        Req: 'req,
        C: ConcurrencyManager<Res>,
    {
        let ctx = self.ctx;

        match result {
            Ok(response) => AwaitResponseTransition::Response(Response {
                response,
                ctx,
                span: Span::none(),
            }),
            Err(ref concurrency_error) => {
                match concurrency_error {
                    ConcurrencyError::Lagged(n) => {
                        debug!(
                            "Concurrency channel lagged by {} messages, falling back to upstream",
                            n
                        );
                    }
                    ConcurrencyError::Closed => {
                        debug!(
                            "Concurrency channel closed, cleaning up stale entry and falling back to upstream"
                        );
                        concurrency_manager.cleanup(&self.cache_key);
                    }
                }

                let upstream_future = self.upstream.call(self.request);

                AwaitResponseTransition::PollUpstream {
                    upstream_future,
                    ctx,
                    cache_key: self.cache_key,
                }
            }
        }
    }
}

impl<Req, U> std::fmt::Debug for AwaitResponse<Req, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwaitResponse")
            .field("cache_key", &self.cache_key)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// UpdateCache
// =============================================================================

/// Data for UpdateCache state (non-pinned part).
///
/// The update cache future is stored separately in the State enum to allow pinning.
/// When the future completes, this data is taken and passed to transition().
pub struct UpdateCache {
    /// Tracing span for this state (created on entry, entered on each poll).
    pub span: Span,
}

impl UpdateCache {
    /// Create a new UpdateCache state with its tracing span.
    pub fn new(parent: &Span) -> Self {
        Self {
            span: span!(parent: parent, Level::TRACE, "fsm.UpdateCache"),
        }
    }

    /// Transition from UpdateCache state after future completes.
    pub fn transition<Res>(self, response: Res, ctx: BoxContext) -> UpdateCacheTransition<Res> {
        UpdateCacheTransition::Response(Response {
            response,
            ctx,
            span: Span::none(),
        })
    }
}

impl std::fmt::Debug for UpdateCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateCache").finish_non_exhaustive()
    }
}
