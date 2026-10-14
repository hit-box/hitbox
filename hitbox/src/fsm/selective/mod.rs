//! Selective cache future for multi-config routing.
//!
//! [`SelectiveCacheFuture`] evaluates request predicates against multiple
//! configurations, selects the first match, and delegates to [`CacheFuture`]
//! starting at the `PollCache` state.

pub(crate) mod states;
pub mod transitions;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{self, Poll};

use futures::ready;
use hitbox_core::{CacheConfigs, Cacheable, Offload, Predicate as _, Upstream};
use pin_project::pin_project;
use tracing::{Level, Span, debug, span, trace};

use crate::backend::CacheBackend;
use crate::concurrency::ConcurrencyManager;
use crate::fsm::CacheFuture;
use crate::policy::PolicyConfig;
use crate::{CacheConfig, CacheContext, CacheableRequest, CacheableResponse};

use states::{CheckPredicate, Passthrough, SelectiveState, SelectiveStateProj};

const TAKE_ERROR: &str = "SelectiveCacheFuture: value already taken";
const POLL_AFTER_READY: &str = "SelectiveCacheFuture: polled after completion";

// =============================================================================
// Type aliases for CacheConfigs projections
// =============================================================================

pub(crate) type ResSubject<Res> = <Res as CacheableResponse>::Subject;
type ConfigOf<CC, Req, Res> = <CC as CacheConfigs<Req, ResSubject<Res>>>::Config;
type ReqPredOf<CC, Req, Res> =
    <ConfigOf<CC, Req, Res> as CacheConfig<Req, ResSubject<Res>>>::RequestPredicate;
type ResPredOf<CC, Req, Res> =
    <ConfigOf<CC, Req, Res> as CacheConfig<Req, ResSubject<Res>>>::ResponsePredicate;
type ExtractorOf<CC, Req, Res> =
    <ConfigOf<CC, Req, Res> as CacheConfig<Req, ResSubject<Res>>>::Extractor;
type ReqTagExtractorOf<CC, Req, Res> =
    <ConfigOf<CC, Req, Res> as CacheConfig<Req, ResSubject<Res>>>::RequestTagExtractor;
type ResTagExtractorOf<CC, Req, Res> =
    <ConfigOf<CC, Req, Res> as CacheConfig<Req, ResSubject<Res>>>::ResponseTagExtractor;

/// Type alias for the inner CacheFuture constructed by the selective FSM.
#[allow(clippy::type_complexity)]
pub(crate) type InnerCacheFuture<'offload, B, Req, Res, CC, U, CM, O> = CacheFuture<
    'offload,
    B,
    Req,
    Res,
    U,
    ReqPredOf<CC, Req, Res>,
    ResPredOf<CC, Req, Res>,
    ExtractorOf<CC, Req, Res>,
    CM,
    O,
    ReqTagExtractorOf<CC, Req, Res>,
    ResTagExtractorOf<CC, Req, Res>,
>;

/// Future that selectively applies caching based on multi-config predicate matching.
///
/// Evaluates request predicates against a sequence of cache configurations
/// and delegates to [`CacheFuture`] for the first matching configuration.
/// If no configuration matches, the request passes through directly to upstream.
///
/// ## Routing Strategy
///
/// **First match wins**: configurations are evaluated in order. The first
/// configuration whose request predicates return `Cacheable` handles the
/// entire request lifecycle. Configurations with [`PolicyConfig::Disabled`]
/// are skipped.
///
/// ## State Flow
///
/// ```text
/// CheckPredicate[0] ─── Cacheable ───► ExtractKey ──► RunCacheFuture
///       │
///   NonCacheable
///       │
///       ▼
/// CheckPredicate[1] ─── Cacheable ───► ExtractKey ──► RunCacheFuture
///       │
///   NonCacheable
///       │
///       ▼
///    Passthrough (upstream.call directly)
/// ```
#[pin_project(project = SelectiveCacheFutureProj)]
pub struct SelectiveCacheFuture<'offload, B, Req, Res, U, CC, CM, O>
where
    U: Upstream<Req, Response = Res>,
    B: CacheBackend,
    Res: CacheableResponse,
    Res::Cached: Send,
    Req: CacheableRequest + Send,
    CC: CacheConfigs<Req, ResSubject<Res>>,
    CM: ConcurrencyManager<Res>,
    O: Offload<'offload>,
{
    configs: CC,
    backend: Arc<B>,
    upstream: Option<U>,
    offload: Option<O>,
    concurrency_manager: Option<CM>,
    #[pin]
    #[allow(clippy::type_complexity)]
    state: SelectiveState<
        'offload,
        InnerCacheFuture<'offload, B, Req, Res, CC, U, CM, O>,
        Req,
        U::Future,
    >,
    /// Parent span for the entire selective cache operation.
    span: Span,
}

impl<'offload, B, Req, Res, U, CC, CM, O> SelectiveCacheFuture<'offload, B, Req, Res, U, CC, CM, O>
where
    U: Upstream<Req, Response = Res>,
    B: CacheBackend,
    Res: CacheableResponse,
    Res::Cached: Send,
    Req: CacheableRequest + Send + 'offload,
    CC: CacheConfigs<Req, ResSubject<Res>>,
    CM: ConcurrencyManager<Res>,
    O: Offload<'offload>,
{
    /// Create a new selective cache future from a [`CacheConfigs`] implementation.
    ///
    /// Evaluates request predicates from each enabled config in order,
    /// delegating to [`CacheFuture`] for the first match.
    /// Configs with [`PolicyConfig::Disabled`] are skipped.
    ///
    /// If no config matches, the request passes through to upstream directly.
    pub fn new(
        configs: CC,
        backend: Arc<B>,
        request: Req,
        upstream: U,
        offload: O,
        concurrency_manager: CM,
    ) -> Self {
        let span = span!(
            Level::DEBUG,
            "hitbox.selective",
            configs = configs.configs().len()
        );

        let first_enabled = configs
            .configs()
            .iter()
            .enumerate()
            .find(|(_, c)| matches!(*c.policy(), PolicyConfig::Enabled(_)))
            .map(|(i, _)| i);

        match first_enabled {
            Some(idx) => {
                let pred = configs.configs()[idx].request_predicates();
                let predicate_future = Box::pin(async move { pred.check(request).await });
                trace!(parent: &span, config_index = idx, "Checking first enabled config");
                SelectiveCacheFuture {
                    configs,
                    backend,
                    upstream: Some(upstream),
                    offload: Some(offload),
                    concurrency_manager: Some(concurrency_manager),
                    state: SelectiveState::CheckPredicate {
                        predicate_future,
                        state: Some(CheckPredicate::new(idx, &span)),
                    },
                    span,
                }
            }
            None => {
                debug!(parent: &span, "No enabled configs, passing through to upstream");
                let upstream_future = upstream.call(request);
                SelectiveCacheFuture {
                    configs,
                    backend,
                    upstream: None,
                    offload: Some(offload),
                    concurrency_manager: Some(concurrency_manager),
                    state: SelectiveState::Passthrough {
                        upstream_future,
                        state: Some(Passthrough::new(&span)),
                    },
                    span,
                }
            }
        }
    }
}

impl<'offload, B, Req, Res, U, CC, CM, O> Future
    for SelectiveCacheFuture<'offload, B, Req, Res, U, CC, CM, O>
where
    U: Upstream<Req, Response = Res> + Send + 'offload,
    U::Future: Send + 'offload,
    B: CacheBackend + Send + Sync + 'static,
    Res: CacheableResponse + Send + 'static,
    Res::Cached: Cacheable + Send,
    Req: CacheableRequest + Send + 'offload,
    CC: CacheConfigs<Req, ResSubject<Res>>,
    CM: ConcurrencyManager<Res> + 'static,
    O: Offload<'offload>,
{
    type Output = (Res, CacheContext);

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            let new_state = match this.state.as_mut().project() {
                SelectiveStateProj::CheckPredicate {
                    predicate_future,
                    state,
                } => {
                    let check = state.as_ref().expect(POLL_AFTER_READY);
                    trace!(parent: &check.span, "FSM state: CheckPredicate");
                    let result = ready!(predicate_future.poll(cx));
                    let check = state.take().expect(POLL_AFTER_READY);
                    check
                        .transition::<_, Res, _, _>(result, &*this.configs, &mut *this.upstream)
                        .into_state(&*this.span)
                }
                SelectiveStateProj::ExtractKey {
                    extract_future,
                    state,
                } => {
                    let extract = state.as_ref().expect(POLL_AFTER_READY);
                    trace!(parent: &extract.span, "FSM state: ExtractKey");
                    let (request, cache_key, request_tags) = ready!(extract_future.poll(cx));
                    let extract = state.take().expect(POLL_AFTER_READY);
                    extract
                        .transition(
                            request,
                            cache_key,
                            request_tags,
                            &*this.configs,
                            this.backend.clone(),
                            &mut *this.upstream,
                            &mut *this.offload,
                            &mut *this.concurrency_manager,
                        )
                        .into_state()
                }
                SelectiveStateProj::RunCacheFuture { inner } => {
                    return inner.poll(cx);
                }
                SelectiveStateProj::Passthrough {
                    upstream_future,
                    state,
                } => {
                    let passthrough = state.as_ref().expect(POLL_AFTER_READY);
                    trace!(parent: &passthrough.span, "FSM state: Passthrough");
                    let response = ready!(upstream_future.poll(cx));
                    let ctx = CacheContext::default().boxed();
                    let ctx = hitbox_core::finalize_context(ctx);
                    return Poll::Ready((response, ctx));
                }
            };

            this.state.set(new_state);
        }
    }
}
