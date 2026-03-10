//! Selective FSM state types and state structs.
//!
//! Each state struct represents resolved async data and has a `.transition()` method
//! that returns the appropriate transition enum. The transition enum then has
//! `.into_state()` to convert to the outer `SelectiveState` enum.
//!
//! Flow: poll future → create state struct → `.transition()` → `.into_state()`

use std::sync::Arc;

use futures::future::BoxFuture;
use hitbox_core::{
    CacheConfigs, Cacheable, Extractor as _, KeyParts, Offload, Predicate as _, PredicateResult,
    Upstream,
};
use pin_project::pin_project;
use tracing::{Level, Span, debug, field, span, trace};

use crate::backend::CacheBackend;
use crate::concurrency::ConcurrencyManager;
use crate::fsm::CacheFuture;
use crate::policy::PolicyConfig;
use crate::{CacheConfig, CacheableRequest, CacheableResponse};

use super::transitions::{CheckPredicateTransition, ExtractKeyTransition};
use super::{InnerCacheFuture, ResSubject, TAKE_ERROR};

// =============================================================================
// SelectiveState Enum
// =============================================================================

/// Internal state machine for [`super::SelectiveCacheFuture`].
#[allow(missing_docs)]
#[pin_project(project = SelectiveStateProj)]
pub enum SelectiveState<'a, Inner, Req, UF> {
    /// Checking if current config's request predicates match.
    CheckPredicate {
        #[pin]
        predicate_future: BoxFuture<'a, PredicateResult<Req>>,
        state: Option<CheckPredicate>,
    },
    /// Matched config — extracting cache key.
    ExtractKey {
        #[pin]
        extract_future: BoxFuture<'a, KeyParts<Req>>,
        state: Option<ExtractKey>,
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
        state: Option<Passthrough>,
    },
}

impl<Inner, Req, UF> std::fmt::Debug for SelectiveState<'_, Inner, Req, UF> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckPredicate { .. } => f.write_str("SelectiveState::CheckPredicate"),
            Self::ExtractKey { .. } => f.write_str("SelectiveState::ExtractKey"),
            Self::RunCacheFuture { .. } => f.write_str("SelectiveState::RunCacheFuture"),
            Self::Passthrough { .. } => f.write_str("SelectiveState::Passthrough"),
        }
    }
}

// =============================================================================
// CheckPredicate
// =============================================================================

/// Data for CheckPredicate state (non-pinned part).
///
/// The predicate future is stored separately in the `SelectiveState` enum to allow pinning.
/// When the future completes, this data is taken and passed to `transition()`.
pub struct CheckPredicate {
    /// Index of the config being evaluated.
    pub config_index: usize,
    /// Tracing span for this state.
    pub span: Span,
}

impl CheckPredicate {
    /// Create a new CheckPredicate state with its tracing span.
    pub fn new(config_index: usize, parent: &Span) -> Self {
        Self {
            config_index,
            span: span!(
                parent: parent,
                Level::TRACE,
                "fsm.CheckPredicate",
                selective.config_index = config_index,
                selective.matched = field::Empty,
            ),
        }
    }

    /// Transition from CheckPredicate state after predicate future completes.
    ///
    /// Based on predicate result:
    /// - Cacheable: proceed to ExtractKey for this config
    /// - NonCacheable + next enabled config exists: check next config
    /// - NonCacheable + no more configs: passthrough to upstream
    pub fn transition<'a, Req, Res, CC, U>(
        self,
        result: PredicateResult<Req>,
        configs: &CC,
        upstream: &mut Option<U>,
    ) -> CheckPredicateTransition<'a, Req, U::Future>
    where
        Req: CacheableRequest + Send + 'a,
        Res: CacheableResponse,
        CC: CacheConfigs<Req, ResSubject<Res>>,
        U: Upstream<Req, Response = Res>,
    {
        match result {
            PredicateResult::Cacheable(request) => {
                self.span.record("selective.matched", true);
                trace!(
                    parent: &self.span,
                    config_index = self.config_index,
                    "Config matched, extracting cache key"
                );
                let ext = configs.configs()[self.config_index].extractors();
                let extract_future = Box::pin(async move { ext.get(request).await });
                CheckPredicateTransition::ExtractKey {
                    extract_future,
                    config_index: self.config_index,
                }
            }
            PredicateResult::NonCacheable(request) => {
                self.span.record("selective.matched", false);
                trace!(
                    parent: &self.span,
                    config_index = self.config_index,
                    "Config did not match, trying next"
                );
                let next = configs
                    .configs()
                    .iter()
                    .enumerate()
                    .skip(self.config_index + 1)
                    .find(|(_, c)| matches!(*c.policy(), PolicyConfig::Enabled(_)))
                    .map(|(i, _)| i);

                match next {
                    Some(next_idx) => {
                        let pred = configs.configs()[next_idx].request_predicates();
                        let predicate_future = Box::pin(async move { pred.check(request).await });
                        CheckPredicateTransition::NextConfig {
                            predicate_future,
                            config_index: next_idx,
                        }
                    }
                    None => {
                        debug!(
                            parent: &self.span,
                            "No configs matched, passing through to upstream"
                        );
                        let upstream = upstream.take().expect(TAKE_ERROR);
                        let upstream_future = upstream.call(request);
                        CheckPredicateTransition::Passthrough { upstream_future }
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for CheckPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckPredicate")
            .field("config_index", &self.config_index)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// ExtractKey
// =============================================================================

/// Data for ExtractKey state (non-pinned part).
///
/// The extract future is stored separately in the `SelectiveState` enum to allow pinning.
/// When the future completes, this data is taken and passed to `transition()`.
pub struct ExtractKey {
    /// Index of the matched config.
    pub config_index: usize,
    /// Tracing span for this state.
    pub span: Span,
}

impl ExtractKey {
    /// Create a new ExtractKey state with its tracing span.
    pub fn new(config_index: usize, parent: &Span) -> Self {
        Self {
            config_index,
            span: span!(
                parent: parent,
                Level::TRACE,
                "fsm.ExtractKey",
                selective.config_index = config_index,
                cache.key = field::Empty,
            ),
        }
    }

    /// Transition from ExtractKey state after extract future completes.
    ///
    /// Always delegates to CacheFuture starting at PollCache state.
    #[allow(clippy::type_complexity)]
    pub fn transition<'offload, B, Req, Res, CC, U, CM, O>(
        self,
        key_parts: KeyParts<Req>,
        configs: &CC,
        backend: Arc<B>,
        upstream: &mut Option<U>,
        offload: &mut Option<O>,
        concurrency_manager: &mut Option<CM>,
    ) -> ExtractKeyTransition<InnerCacheFuture<'offload, B, Req, Res, CC, U, CM, O>>
    where
        B: CacheBackend + Send + Sync + 'static,
        Req: CacheableRequest + Send + 'offload,
        Res: CacheableResponse + Send + 'static,
        Res::Cached: Cacheable + Send,
        CC: CacheConfigs<Req, ResSubject<Res>>,
        U: Upstream<Req, Response = Res> + Send + 'offload,
        U::Future: Send + 'offload,
        CM: ConcurrencyManager<Res> + 'static,
        O: Offload<'offload>,
    {
        let (request, cache_key) = key_parts.into_cache_key();
        self.span
            .record("cache.key", cache_key.to_string().as_str());
        debug!(
            parent: &self.span,
            config_index = self.config_index,
            cache.key = %cache_key,
            "Cache key extracted, delegating to CacheFuture"
        );

        let upstream = upstream.take().expect(TAKE_ERROR);
        let response_predicates = configs.configs()[self.config_index].response_predicates();
        let policy = configs.configs()[self.config_index].policy();
        let offload = offload.take().expect(TAKE_ERROR);
        let concurrency_manager = concurrency_manager.take().expect(TAKE_ERROR);

        let inner = CacheFuture::poll_cache(
            backend,
            cache_key,
            request,
            upstream,
            response_predicates,
            policy,
            offload,
            concurrency_manager,
        );

        ExtractKeyTransition::RunCacheFuture { inner }
    }
}

impl std::fmt::Debug for ExtractKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractKey")
            .field("config_index", &self.config_index)
            .finish_non_exhaustive()
    }
}

// =============================================================================
// Passthrough
// =============================================================================

/// Data for Passthrough state (non-pinned part).
///
/// The upstream future is stored separately in the `SelectiveState` enum to allow pinning.
/// This is a terminal state — no transition method, the poll loop returns directly.
pub struct Passthrough {
    /// Tracing span for this state.
    pub span: Span,
}

impl Passthrough {
    /// Create a new Passthrough state with its tracing span.
    pub fn new(parent: &Span) -> Self {
        Self {
            span: span!(parent: parent, Level::TRACE, "fsm.Passthrough"),
        }
    }
}

impl std::fmt::Debug for Passthrough {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Passthrough").finish_non_exhaustive()
    }
}
