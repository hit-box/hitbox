//! Cache future implementation.
//!
//! [`CacheFuture`] drives the FSM through its states, yielding the final response
//! with cache context indicating hit/miss/stale status and response source.

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{self, Poll},
    time::Instant,
};

use crate::{CacheContext, CacheStatus, CacheableResponse, ResponseSource};
use futures::ready;
use hitbox_core::{Cacheable, DisabledOffload, Offload, OffloadKey, Upstream};
use pin_project::pin_project;
use tracing::{Level, Span, debug, span, trace};

use crate::{
    CacheKey, CacheableRequest, Extractor, Predicate,
    backend::CacheBackend,
    concurrency::{ConcurrencyManager, NoopConcurrencyManager},
    fsm::states::{self, PollUpstream, State, StateProj},
};

const POLL_AFTER_READY_ERROR: &str = "CacheFuture can't be polled after finishing";

/// Future that executes the cache FSM and returns a response with cache context.
///
/// Drives the state machine through: request predicate check → cache lookup →
/// upstream call (on miss) → response predicate check → cache update → response.
#[pin_project(project = CacheFutureProj)]
pub struct CacheFuture<'offload, B, Req, Res, U, ReqP, ResP, E, C, O = DisabledOffload>
where
    U: Upstream<Req, Response = Res>,
    B: CacheBackend,
    Res: CacheableResponse,
    Req: CacheableRequest,
    ReqP: Predicate<Subject = Req> + Send + Sync,
    ResP: Predicate<Subject = Res::Subject> + Send + Sync,
    E: Extractor<Subject = Req> + Send + Sync,
    C: ConcurrencyManager<Res>,
    O: Offload<'offload>,
{
    backend: Arc<B>,
    cache_key: Option<CacheKey>,
    #[pin]
    state: State<'offload, Res, Req, U, ReqP, E>,
    response_predicates: Option<ResP>,
    policy: Arc<crate::policy::PolicyConfig>,
    /// Offload for background revalidation (SWR).
    /// Use `DisabledOffload` (the default) to disable background revalidation.
    offload: O,
    /// Whether this is a background revalidation task.
    is_revalidation: bool,
    concurrency_manager: C,
    /// Start time for latency measurement.
    start_time: Instant,
    /// Parent span for the entire cache operation (DEBUG level).
    span: Span,
    /// Phantom lifetime marker for offload spawning.
    _lifetime: std::marker::PhantomData<&'offload ()>,
}

impl<'offload, B, Req, Res, U, ReqP, ResP, E, C, O>
    CacheFuture<'offload, B, Req, Res, U, ReqP, ResP, E, C, O>
where
    U: Upstream<Req, Response = Res>,
    B: CacheBackend,
    Res: CacheableResponse,
    Req: CacheableRequest,
    ReqP: Predicate<Subject = Req> + Send + Sync,
    ResP: Predicate<Subject = Res::Subject> + Send + Sync,
    E: Extractor<Subject = Req> + Send + Sync,
    C: ConcurrencyManager<Res>,
    O: Offload<'offload>,
{
    /// Creates a new cache future with full configuration.
    pub fn new(
        backend: Arc<B>,
        request: Req,
        upstream: U,
        request_predicates: ReqP,
        response_predicates: ResP,
        key_extractors: E,
        policy: Arc<crate::policy::PolicyConfig>,
        offload: O,
        concurrency_manager: C,
    ) -> Self {
        let parent_span = span!(Level::DEBUG, "hitbox.cache");
        let initial_state = states::Initial::new(
            request,
            request_predicates,
            key_extractors,
            CacheContext::default().boxed(),
            upstream,
            &parent_span,
        );
        CacheFuture {
            backend,
            cache_key: None,
            state: State::Initial(Some(initial_state)),
            response_predicates: Some(response_predicates),
            policy,
            offload,
            is_revalidation: false,
            concurrency_manager,
            start_time: Instant::now(),
            span: parent_span,
            _lifetime: std::marker::PhantomData,
        }
    }
}

impl<'offload, B, Req, Res, U, ReqP, ResP, E>
    CacheFuture<
        'offload,
        B,
        Req,
        Res,
        U,
        ReqP,
        ResP,
        E,
        NoopConcurrencyManager,
        hitbox_core::DisabledOffload,
    >
where
    U: Upstream<Req, Response = Res>,
    U::Future: Send + 'offload,
    B: CacheBackend,
    Res: CacheableResponse,
    Req: CacheableRequest,
    ReqP: Predicate<Subject = Req> + Send + Sync,
    ResP: Predicate<Subject = Res::Subject> + Send + Sync,
    E: Extractor<Subject = Req> + Send + Sync,
{
    /// Create a CacheFuture for background revalidation (Stale-While-Revalidate).
    ///
    /// This constructor initializes the FSM at `PollUpstream` state, skipping
    /// the cache lookup phase. Use this when you want to refresh a stale cache
    /// entry in the background.
    ///
    /// # Arguments
    /// * `backend` - Cache backend for storing the refreshed value
    /// * `cache_key` - Key to update in the cache
    /// * `request` - Request to send to upstream
    /// * `upstream` - Upstream service to call
    /// * `response_predicates` - Predicates to check if response should be cached
    /// * `policy` - Cache policy configuration (TTL, stale TTL)
    ///
    /// Note: `request_predicates` and `key_extractors` are not needed for revalidation
    /// since the FSM starts at `PollUpstream` state, skipping the initial request check.
    pub fn revalidate(
        backend: Arc<B>,
        cache_key: CacheKey,
        request: Req,
        upstream: U,
        response_predicates: ResP,
        policy: Arc<crate::policy::PolicyConfig>,
    ) -> Self {
        let upstream_future = upstream.call(request);
        let parent_span = span!(Level::DEBUG, "hitbox.cache.revalidate");
        let (state, instrumented_future) = PollUpstream::with_future(
            None,
            CacheContext::default().boxed(),
            Some(cache_key.clone()),
            upstream_future,
            &parent_span,
        );

        CacheFuture {
            backend,
            cache_key: Some(cache_key),
            state: State::PollUpstream {
                upstream_future: instrumented_future,
                state: Some(state),
            },
            response_predicates: Some(response_predicates),
            policy,
            // Revalidation tasks don't spawn further revalidation
            offload: DisabledOffload,
            is_revalidation: true,
            // Revalidation tasks don't need concurrency control
            concurrency_manager: NoopConcurrencyManager,
            start_time: Instant::now(),
            span: parent_span,
            _lifetime: std::marker::PhantomData,
        }
    }
}

impl<'offload, B, Req, Res, U, ReqP, ResP, E, C, O>
    CacheFuture<'offload, B, Req, Res, U, ReqP, ResP, E, C, O>
where
    U: Upstream<Req, Response = Res>,
    B: CacheBackend + Send + Sync + 'static,
    Res: CacheableResponse,
    Res::Cached: Cacheable + Send,
    Req: CacheableRequest,
    ReqP: Predicate<Subject = Req> + Send + Sync,
    ResP: Predicate<Subject = Res::Subject> + Send + Sync,
    E: Extractor<Subject = Req> + Send + Sync,
    C: ConcurrencyManager<Res>,
    O: Offload<'offload>,
{
    /// Create a CacheFuture starting at `PollCache` state, skipping predicate
    /// evaluation and key extraction.
    ///
    /// Used by [`MultiCacheFuture`] after the routing FSM has already evaluated
    /// request predicates and extracted the cache key.
    ///
    /// Note: `ReqP` and `E` are phantom types in this path — the FSM starts
    /// past the states that use them (same pattern as [`revalidate`](Self::revalidate)).
    pub fn poll_cache(
        backend: Arc<B>,
        cache_key: CacheKey,
        request: Req,
        upstream: U,
        response_predicates: ResP,
        policy: Arc<crate::policy::PolicyConfig>,
        offload: O,
        concurrency_manager: C,
    ) -> Self {
        let parent_span = span!(Level::DEBUG, "hitbox.cache");
        let cache_key_for_get = cache_key.clone();
        let backend_for_get = backend.clone();
        let mut ctx = CacheContext::default().boxed();
        let poll_cache = Box::pin(async move {
            let result = backend_for_get
                .get::<Res>(&cache_key_for_get, &mut ctx)
                .await;
            debug!(
                found = result.as_ref().map(|r| r.is_some()).unwrap_or(false),
                "FSM cache lookup result"
            );
            (result, ctx)
        });
        let poll_cache_state =
            states::PollCache::new(request, cache_key.clone(), upstream, &parent_span);

        CacheFuture {
            backend,
            cache_key: Some(cache_key),
            state: State::PollCache {
                poll_cache,
                state: Some(poll_cache_state),
            },
            response_predicates: Some(response_predicates),
            policy,
            offload,
            is_revalidation: false,
            concurrency_manager,
            start_time: Instant::now(),
            span: parent_span,
            _lifetime: std::marker::PhantomData,
        }
    }
}

impl<'offload, B, Req, Res, U, ReqP, ResP, E, C, O> Future
    for CacheFuture<'offload, B, Req, Res, U, ReqP, ResP, E, C, O>
where
    U: Upstream<Req, Response = Res> + Send + 'offload,
    U::Future: Send + 'offload,
    B: CacheBackend + Send + Sync + 'static,
    Res: CacheableResponse + Send + 'static,
    Res::Cached: Cacheable + Send,
    Req: CacheableRequest + Send + 'offload,
    ReqP: Predicate<Subject = Req> + Send + Sync + 'offload,
    ResP: Predicate<Subject = Res::Subject> + Send + Sync + 'static,
    E: Extractor<Subject = Req> + Send + Sync + 'offload,
    C: ConcurrencyManager<Res> + 'static,
    O: Offload<'offload>,
{
    type Output = (Res, CacheContext);

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            let state = match this.state.as_mut().project() {
                StateProj::Initial(initial_state) => {
                    let initial = initial_state.take().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &initial.span, "FSM state: Initial");
                    initial
                        .transition(this.policy.as_ref())
                        .into_state(&*this.span)
                }
                StateProj::CheckRequestCachePolicy {
                    cache_policy_future,
                    state,
                } => {
                    let state_ref = state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: CheckRequestCachePolicy");
                    let policy = ready!(cache_policy_future.poll(cx));
                    let check_state = state.take().expect(POLL_AFTER_READY_ERROR);

                    check_state
                        .transition(policy, this.backend.clone(), this.cache_key)
                        .into_state(&*this.span)
                }
                StateProj::PollCache { poll_cache, state } => {
                    let state_ref = state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: PollCache");
                    let (cache_result, ctx) = ready!(poll_cache.poll(cx));
                    let poll_cache_state = state.take().expect(POLL_AFTER_READY_ERROR);

                    poll_cache_state
                        .transition(
                            cache_result,
                            ctx,
                            this.backend.clone(),
                            this.policy.as_ref(),
                            &*this.concurrency_manager,
                        )
                        .into_state(&*this.span)
                }
                StateProj::AwaitResponse {
                    await_response_future,
                    state,
                } => {
                    let state_ref = state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: AwaitResponse");
                    let result = ready!(await_response_future.poll(cx));
                    let await_response_state = state.take().expect(POLL_AFTER_READY_ERROR);

                    await_response_state
                        .transition(result, &*this.concurrency_manager)
                        .into_state(&*this.span)
                }
                StateProj::ConvertResponse {
                    response_future,
                    state,
                } => {
                    let state_ref = state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: ConvertResponse");
                    let (response, ctx) = ready!(response_future.poll(cx));
                    let convert_response_state = state.take().expect(POLL_AFTER_READY_ERROR);
                    convert_response_state
                        .transition(response, ctx)
                        .into_state(&*this.span)
                }
                StateProj::HandleStale {
                    response_future,
                    state,
                } => {
                    let state_ref = state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: HandleStale");
                    let (response, ctx) = ready!(response_future.poll(cx));
                    let handle_stale_state = state.take().expect(POLL_AFTER_READY_ERROR);

                    let result = handle_stale_state.transition(response, ctx, this.policy.as_ref());

                    // Handle offload revalidation if requested
                    // Note: DisabledOffload::register is a no-op, so this does nothing when offload is disabled
                    if let Some(offload_data) = result.offload_data
                        && let Some(response_predicates) = this.response_predicates.take()
                    {
                        let backend = this.backend.clone();
                        let policy = this.policy.clone();
                        let cache_key = offload_data.cache_key.clone();
                        let request = offload_data.request;
                        let upstream = offload_data.upstream;

                        // Create revalidation future using the existing FSM
                        // ReqP and E are phantom types in revalidation path
                        let revalidate_future: CacheFuture<'_, _, _, _, _, ReqP, _, E, _, _> =
                            CacheFuture::revalidate(
                                backend,
                                cache_key.clone(),
                                request,
                                upstream,
                                response_predicates,
                                policy,
                            );

                        // Use register with Keyed key for proper deduplication
                        // This ensures only one revalidation task runs per cache key
                        this.offload.register(
                            OffloadKey::keyed(cache_key, "revalidate"),
                            async move {
                                let _ = revalidate_future.await;
                            },
                        );
                    }

                    result.transition.into_state(&*this.span)
                }
                StateProj::PollUpstream {
                    upstream_future,
                    state,
                } => {
                    let state_ref = state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: PollUpstream");
                    let upstream_result = ready!(upstream_future.poll(cx));
                    let poll_upstream = state.take().expect(POLL_AFTER_READY_ERROR);
                    let predicates = this
                        .response_predicates
                        .take()
                        .expect("Response predicates already taken");

                    poll_upstream
                        .transition(upstream_result, predicates, this.policy.as_ref())
                        .into_state(&*this.span)
                }
                StateProj::CheckResponseCachePolicy {
                    cache_policy,
                    state,
                } => {
                    let state_ref = state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: CheckResponseCachePolicy");
                    let policy = ready!(cache_policy.poll(cx));
                    let check_state = state.take().expect(POLL_AFTER_READY_ERROR);

                    check_state
                        .transition(policy, this.backend.clone(), &*this.concurrency_manager)
                        .into_state(&*this.span)
                }
                StateProj::UpdateCache {
                    update_cache_future,
                    state,
                } => {
                    let state_ref = state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: UpdateCache");
                    // TODO: check backend result
                    let (_backend_result, response, ctx) = ready!(update_cache_future.poll(cx));
                    let update_cache_state = state.take().expect(POLL_AFTER_READY_ERROR);
                    update_cache_state
                        .transition(response, ctx)
                        .into_state(&*this.span)
                }
                StateProj::Response(response_state) => {
                    let state_ref = response_state.as_ref().expect(POLL_AFTER_READY_ERROR);
                    trace!(parent: &state_ref.span, "FSM state: Response");
                    let mut state = response_state.take().expect(POLL_AFTER_READY_ERROR);
                    // For cache miss, set source to Upstream.
                    // For hit/stale, the backend has already set the correct source.
                    if state.ctx.status() == CacheStatus::Miss {
                        state.ctx.set_source(ResponseSource::Upstream);
                    }
                    let ctx = hitbox_core::finalize_context(state.ctx);
                    // Record final status and source to span
                    state.span.record("cache.status", ctx.status.as_str());
                    state.span.record("cache.source", ctx.source.as_str());
                    let duration = this.start_time.elapsed();
                    crate::metrics::record_context_metrics(&ctx, duration, *this.is_revalidation);
                    debug!(parent: &*this.span, status = ?ctx.status, source = ?ctx.source, "Cache operation completed");
                    return Poll::Ready((state.response, ctx));
                }
            };
            this.state.set(state);
        }
    }
}
