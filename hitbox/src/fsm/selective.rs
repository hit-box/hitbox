//! Selective cache future for multi-config routing.
//!
//! [`SelectiveCacheFuture`] evaluates request predicates against multiple
//! configurations, selects the first match, and delegates to [`CacheFuture`]
//! starting at the `PollCache` state.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{self, Poll};

use futures::future::BoxFuture;
use futures::ready;
use hitbox_core::{CacheConfigs, Cacheable, Offload, PredicateResult, Upstream};
use pin_project::pin_project;
use tracing::{Level, Span, debug, span, trace};

use crate::{
    CacheConfig, CacheContext, CacheableRequest, CacheableResponse, Extractor, Predicate,
    backend::CacheBackend, concurrency::ConcurrencyManager, fsm::CacheFuture, policy::PolicyConfig,
};

const TAKE_ERROR: &str = "SelectiveCacheFuture: value already taken";

// =============================================================================
// Type aliases for CacheConfigs projections
// =============================================================================

type ResSubject<Res> = <Res as CacheableResponse>::Subject;
type ConfigOf<CC, Req, Res> = <CC as CacheConfigs<Req, ResSubject<Res>>>::Config;
type ReqPredOf<CC, Req, Res> =
    <ConfigOf<CC, Req, Res> as CacheConfig<Req, ResSubject<Res>>>::RequestPredicate;
type ResPredOf<CC, Req, Res> =
    <ConfigOf<CC, Req, Res> as CacheConfig<Req, ResSubject<Res>>>::ResponsePredicate;
type ExtractorOf<CC, Req, Res> =
    <ConfigOf<CC, Req, Res> as CacheConfig<Req, ResSubject<Res>>>::Extractor;

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
    Req: CacheableRequest,
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
        CacheFuture<
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
        >,
        Req,
        U::Future,
    >,
    /// Parent span for the entire selective cache operation.
    span: Span,
}

/// Internal state machine for [`SelectiveCacheFuture`].
#[pin_project(project = SelectiveStateProj)]
enum SelectiveState<'a, Inner, Req, UF> {
    /// Checking if current config's request predicates match.
    CheckPredicate {
        #[pin]
        predicate_future: BoxFuture<'a, PredicateResult<Req>>,
        config_index: usize,
    },
    /// Matched config — extracting cache key.
    ExtractKey {
        #[pin]
        extract_future: BoxFuture<'a, hitbox_core::KeyParts<Req>>,
        config_index: usize,
    },
    /// Running inner CacheFuture from PollCache state.
    RunCacheFuture {
        #[pin]
        inner: Inner,
    },
    /// No config matched — calling upstream directly.
    Passthrough {
        #[pin]
        upstream_future: UF,
    },
}

impl<'offload, B, Req, Res, U, CC, CM, O> SelectiveCacheFuture<'offload, B, Req, Res, U, CC, CM, O>
where
    U: Upstream<Req, Response = Res>,
    B: CacheBackend,
    Res: CacheableResponse,
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
            .find(|(_, c)| matches!(c.policy(), PolicyConfig::Enabled(_)))
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
                        config_index: idx,
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
                    state: SelectiveState::Passthrough { upstream_future },
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
                    config_index,
                } => {
                    let result = ready!(predicate_future.poll(cx));
                    let idx = *config_index;
                    let _check_span = span!(
                        parent: &*this.span,
                        Level::TRACE,
                        "fsm.CheckPredicate",
                        selective.config_index = idx,
                        selective.matched = matches!(result, PredicateResult::Cacheable(_))
                    );

                    match result {
                        PredicateResult::Cacheable(request) => {
                            trace!(
                                parent: &*this.span,
                                config_index = idx,
                                "Config matched, extracting cache key"
                            );
                            let ext = this.configs.configs()[idx].extractors();
                            let extract_future = Box::pin(async move { ext.get(request).await });
                            SelectiveState::ExtractKey {
                                extract_future,
                                config_index: idx,
                            }
                        }
                        PredicateResult::NonCacheable(request) => {
                            trace!(
                                parent: &*this.span,
                                config_index = idx,
                                "Config did not match, trying next"
                            );
                            let next = this
                                .configs
                                .configs()
                                .iter()
                                .enumerate()
                                .skip(idx + 1)
                                .find(|(_, c)| matches!(c.policy(), PolicyConfig::Enabled(_)))
                                .map(|(i, _)| i);

                            match next {
                                Some(next_idx) => {
                                    let pred =
                                        this.configs.configs()[next_idx].request_predicates();
                                    let predicate_future =
                                        Box::pin(async move { pred.check(request).await });
                                    SelectiveState::CheckPredicate {
                                        predicate_future,
                                        config_index: next_idx,
                                    }
                                }
                                None => {
                                    debug!(
                                        parent: &*this.span,
                                        "No configs matched, passing through to upstream"
                                    );
                                    let upstream = this.upstream.take().expect(TAKE_ERROR);
                                    let upstream_future = upstream.call(request);
                                    SelectiveState::Passthrough { upstream_future }
                                }
                            }
                        }
                    }
                }
                SelectiveStateProj::ExtractKey {
                    extract_future,
                    config_index,
                } => {
                    let key_parts = ready!(extract_future.poll(cx));
                    let idx = *config_index;
                    let (request, cache_key) = key_parts.into_cache_key();
                    let _extract_span = span!(
                        parent: &*this.span,
                        Level::TRACE,
                        "fsm.ExtractKey",
                        selective.config_index = idx,
                        cache.key = %cache_key
                    );

                    debug!(
                        parent: &*this.span,
                        config_index = idx,
                        cache.key = %cache_key,
                        "Cache key extracted, delegating to CacheFuture"
                    );

                    let upstream = this.upstream.take().expect(TAKE_ERROR);
                    let response_predicates = this.configs.configs()[idx].response_predicates();
                    let policy = Arc::new(this.configs.configs()[idx].policy().clone());
                    let offload = this.offload.take().expect(TAKE_ERROR);
                    let concurrency_manager = this.concurrency_manager.take().expect(TAKE_ERROR);

                    let inner = CacheFuture::poll_cache(
                        this.backend.clone(),
                        cache_key,
                        request,
                        upstream,
                        response_predicates,
                        policy,
                        offload,
                        concurrency_manager,
                    );

                    SelectiveState::RunCacheFuture { inner }
                }
                SelectiveStateProj::RunCacheFuture { inner } => {
                    return inner.poll(cx);
                }
                SelectiveStateProj::Passthrough { upstream_future } => {
                    let response = ready!(upstream_future.poll(cx));
                    let _passthrough_span = span!(
                        parent: &*this.span,
                        Level::TRACE,
                        "fsm.Passthrough"
                    );
                    let ctx = CacheContext::default().boxed();
                    let ctx = hitbox_core::finalize_context(ctx);
                    return Poll::Ready((response, ctx));
                }
            };

            this.state.set(new_state);
        }
    }
}
