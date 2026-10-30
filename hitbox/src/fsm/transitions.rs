//! FSM transition types.
//!
//! Transition enums represent the possible outcomes from each state's `.transition()` method.
//! Each transition enum has an `.into_state()` method to convert to the outer `State` enum.
//!
//! These are internal implementation details — see [`crate::fsm`] for the public API.

#![allow(missing_docs)]

use futures::future::BoxFuture;
use hitbox_core::{BoxContext, ResponseCachePolicy, Upstream};
use tokio::sync::OwnedSemaphorePermit;
use tracing::Span;

use crate::fsm::states::{
    AwaitResponse, AwaitResponseFuture, CheckRequestCachePolicy, CheckResponseCachePolicy,
    ConvertResponse, ConvertResponseFuture, HandleStale, PollCache, PollCacheWithTagsFuture,
    PollUpstream, RequestCachePolicyFuture, Response, State, UpdateCache, UpdateCacheFuture,
};
use crate::{CacheKey, CacheableRequest, CacheableResponse, Extractor, Predicate};
use hitbox_core::tag::TagExtractor;

// =============================================================================
// InitialTransition
// =============================================================================

/// Transitions from Initial state.
pub enum InitialTransition<'req, Req, U>
where
    Req: CacheableRequest + 'req,
    U: Upstream<Req> + 'req,
{
    /// Cache is enabled - check request cache policy
    CheckRequestCachePolicy {
        cache_policy_future: RequestCachePolicyFuture<'req, Req>,
        ctx: BoxContext,
        upstream: U,
    },
    /// Cache is disabled - poll upstream directly
    PollUpstream {
        upstream_future: U::Future,
        ctx: BoxContext,
    },
}

impl<'req, Req, U> InitialTransition<'req, Req, U>
where
    Req: CacheableRequest + 'req,
    U: Upstream<Req> + 'req,
{
    pub fn into_state<Res, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Res: CacheableResponse,
        U: Upstream<Req, Response = Res>,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            InitialTransition::CheckRequestCachePolicy {
                cache_policy_future,
                ctx,
                upstream,
            } => State::CheckRequestCachePolicy {
                cache_policy_future,
                state: Some(CheckRequestCachePolicy::new(ctx, upstream, parent)),
            },
            InitialTransition::PollUpstream {
                upstream_future,
                ctx,
            } => {
                let (state, instrumented_future) =
                    PollUpstream::with_future(None, ctx, None, upstream_future, parent);
                State::PollUpstream {
                    upstream_future: instrumented_future,
                    state: Some(state),
                }
            }
        }
    }
}

impl<Req, U> std::fmt::Debug for InitialTransition<'_, Req, U>
where
    Req: CacheableRequest,
    U: Upstream<Req>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckRequestCachePolicy { .. } => {
                f.write_str("InitialTransition::CheckRequestCachePolicy")
            }
            Self::PollUpstream { .. } => f.write_str("InitialTransition::PollUpstream"),
        }
    }
}

// =============================================================================
// CheckRequestCachePolicyTransition
// =============================================================================

/// Transitions from CheckRequestCachePolicy state.
pub enum CheckRequestCachePolicyTransition<Req, Res, U>
where
    Res: CacheableResponse,
    U: Upstream<Req, Response = Res>,
{
    /// Request is cacheable - poll cache (with parallel tag invalidation check)
    PollCache {
        poll_cache: PollCacheWithTagsFuture<Res::Cached>,
        request: Req,
        cache_key: CacheKey,
        upstream: U,
    },
    /// Request is not cacheable - poll upstream directly
    PollUpstream {
        upstream_future: U::Future,
        ctx: BoxContext,
    },
}

impl<Req, Res, U> CheckRequestCachePolicyTransition<Req, Res, U>
where
    Res: CacheableResponse,
    U: Upstream<Req, Response = Res>,
{
    pub fn into_state<'req, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Req: CacheableRequest + 'req,
        U: 'req,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            CheckRequestCachePolicyTransition::PollCache {
                poll_cache,
                request,
                cache_key,
                upstream,
            } => State::PollCache {
                poll_cache,
                state: Some(PollCache::new(request, cache_key, upstream, parent)),
            },
            CheckRequestCachePolicyTransition::PollUpstream {
                upstream_future,
                ctx,
            } => {
                let (state, instrumented_future) =
                    PollUpstream::with_future(None, ctx, None, upstream_future, parent);
                State::PollUpstream {
                    upstream_future: instrumented_future,
                    state: Some(state),
                }
            }
        }
    }
}

impl<Req, Res, U> std::fmt::Debug for CheckRequestCachePolicyTransition<Req, Res, U>
where
    Res: CacheableResponse,
    U: Upstream<Req, Response = Res>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PollCache { .. } => f.write_str("CheckRequestCachePolicyTransition::PollCache"),
            Self::PollUpstream { .. } => {
                f.write_str("CheckRequestCachePolicyTransition::PollUpstream")
            }
        }
    }
}

// =============================================================================
// PollCacheTransition
// =============================================================================

/// Transitions from PollCache state.
pub enum PollCacheTransition<Res, Req, U>
where
    Res: CacheableResponse,
    U: Upstream<Req, Response = Res>,
{
    /// Cache hit (actual) with refill needed - update cache then return
    UpdateCache {
        update_cache_future: UpdateCacheFuture<Res>,
    },
    /// Cache hit (actual) - convert to response
    ConvertResponse {
        response_future: ConvertResponseFuture<Res>,
        cache_key: CacheKey,
    },
    /// Cache hit (stale) - handle stale policy
    HandleStale {
        response_future: ConvertResponseFuture<Res>,
        request: Req,
        cache_key: CacheKey,
        upstream: U,
    },
    /// Cache miss/expired - poll upstream directly
    PollUpstream {
        upstream_future: U::Future,
        permit: Option<OwnedSemaphorePermit>,
        ctx: BoxContext,
        cache_key: CacheKey,
    },
    /// Cache miss/expired with concurrency - await another request's response
    AwaitResponse {
        await_response_future: AwaitResponseFuture<Res>,
        request: Req,
        ctx: BoxContext,
        cache_key: CacheKey,
        upstream: U,
    },
}

impl<Res, Req, U> PollCacheTransition<Res, Req, U>
where
    Res: CacheableResponse,
    U: Upstream<Req, Response = Res>,
{
    pub fn into_state<'req, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Req: CacheableRequest + 'req,
        U: 'req,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            PollCacheTransition::UpdateCache {
                update_cache_future,
            } => State::UpdateCache {
                update_cache_future,
                state: Some(UpdateCache::new(parent)),
            },
            PollCacheTransition::ConvertResponse {
                response_future,
                cache_key,
            } => State::ConvertResponse {
                response_future,
                state: Some(ConvertResponse::new(cache_key, parent)),
            },
            PollCacheTransition::HandleStale {
                response_future,
                request,
                cache_key,
                upstream,
            } => State::HandleStale {
                response_future,
                state: Some(HandleStale::new(request, cache_key, upstream, parent)),
            },
            PollCacheTransition::PollUpstream {
                upstream_future,
                permit,
                ctx,
                cache_key,
            } => {
                let (state, instrumented_future) = PollUpstream::with_future(
                    permit,
                    ctx,
                    Some(cache_key),
                    upstream_future,
                    parent,
                );
                State::PollUpstream {
                    upstream_future: instrumented_future,
                    state: Some(state),
                }
            }
            PollCacheTransition::AwaitResponse {
                await_response_future,
                request,
                ctx,
                cache_key,
                upstream,
            } => State::AwaitResponse {
                await_response_future,
                state: Some(AwaitResponse::new(
                    request, ctx, cache_key, upstream, parent,
                )),
            },
        }
    }
}

impl<Res, Req, U> std::fmt::Debug for PollCacheTransition<Res, Req, U>
where
    Res: CacheableResponse,
    U: Upstream<Req, Response = Res>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpdateCache { .. } => f.write_str("PollCacheTransition::UpdateCache"),
            Self::ConvertResponse { .. } => f.write_str("PollCacheTransition::ConvertResponse"),
            Self::HandleStale { .. } => f.write_str("PollCacheTransition::HandleStale"),
            Self::PollUpstream { .. } => f.write_str("PollCacheTransition::PollUpstream"),
            Self::AwaitResponse { .. } => f.write_str("PollCacheTransition::AwaitResponse"),
        }
    }
}

// =============================================================================
// ConvertResponseTransition
// =============================================================================

/// Transitions from ConvertResponse state.
pub enum ConvertResponseTransition<Res> {
    Response(Response<Res>),
}

impl<Res> ConvertResponseTransition<Res> {
    pub fn into_state<'req, Req, U, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Res: CacheableResponse,
        Req: CacheableRequest + 'req,
        U: Upstream<Req, Response = Res> + 'req,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            ConvertResponseTransition::Response(s) => {
                State::Response(Some(Response::new(s.response, s.ctx, parent)))
            }
        }
    }
}

impl<Res> std::fmt::Debug for ConvertResponseTransition<Res> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Response(_) => f.write_str("ConvertResponseTransition::Response"),
        }
    }
}

// =============================================================================
// HandleStaleTransition
// =============================================================================

/// Transitions from HandleStale state.
///
/// Note: `Req` type parameter is needed for the `Upstream` trait bound, even though
/// it's not directly used in the enum variants after removing `ResponseWithOffload`.
pub enum HandleStaleTransition<Res, Req, U>
where
    U: Upstream<Req, Response = Res>,
{
    /// Return stale response immediately (includes offload case - spawning handled in transition)
    Response(Response<Res>),
    /// Revalidate synchronously - block and wait for fresh data
    Revalidate {
        upstream_future: U::Future,
        ctx: BoxContext,
        cache_key: CacheKey,
    },
}

impl<Res, Req, U> HandleStaleTransition<Res, Req, U>
where
    Res: CacheableResponse,
    U: Upstream<Req, Response = Res>,
{
    pub fn into_state<'req, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Req: CacheableRequest + 'req,
        U: 'req,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            HandleStaleTransition::Response(s) => {
                State::Response(Some(Response::new(s.response, s.ctx, parent)))
            }
            HandleStaleTransition::Revalidate {
                upstream_future,
                ctx,
                cache_key,
            } => {
                let (state, instrumented_future) =
                    PollUpstream::with_future(None, ctx, Some(cache_key), upstream_future, parent);
                State::PollUpstream {
                    upstream_future: instrumented_future,
                    state: Some(state),
                }
            }
        }
    }
}

impl<Res, Req, U> std::fmt::Debug for HandleStaleTransition<Res, Req, U>
where
    U: Upstream<Req, Response = Res>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Response(_) => f.write_str("HandleStaleTransition::Response"),
            Self::Revalidate { .. } => f.write_str("HandleStaleTransition::Revalidate"),
        }
    }
}

// =============================================================================
// AwaitResponseTransition
// =============================================================================

/// Transitions from AwaitResponse state.
pub enum AwaitResponseTransition<Res, Req, U>
where
    U: Upstream<Req, Response = Res>,
{
    Response(Response<Res>),
    PollUpstream {
        upstream_future: U::Future,
        ctx: BoxContext,
        cache_key: CacheKey,
    },
}

impl<Res, Req, U> AwaitResponseTransition<Res, Req, U>
where
    Res: CacheableResponse,
    U: Upstream<Req, Response = Res>,
{
    pub fn into_state<'req, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Req: CacheableRequest + 'req,
        U: 'req,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            AwaitResponseTransition::Response(s) => {
                State::Response(Some(Response::new(s.response, s.ctx, parent)))
            }
            AwaitResponseTransition::PollUpstream {
                upstream_future,
                ctx,
                cache_key,
            } => {
                let (state, instrumented_future) =
                    PollUpstream::with_future(None, ctx, Some(cache_key), upstream_future, parent);
                State::PollUpstream {
                    upstream_future: instrumented_future,
                    state: Some(state),
                }
            }
        }
    }
}

impl<Res, Req, U> std::fmt::Debug for AwaitResponseTransition<Res, Req, U>
where
    U: Upstream<Req, Response = Res>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Response(_) => f.write_str("AwaitResponseTransition::Response"),
            Self::PollUpstream { .. } => f.write_str("AwaitResponseTransition::PollUpstream"),
        }
    }
}

// =============================================================================
// PollUpstreamTransition
// =============================================================================

/// Transitions from PollUpstream state.
pub enum PollUpstreamTransition<Res>
where
    Res: CacheableResponse,
{
    /// Proceed to check response cache policy.
    ///
    /// The future returns both the cache policy and any response tags
    /// extracted from the upstream response (`Vec::new()` when no
    /// response tag extractor is configured).
    CheckResponseCachePolicy {
        cache_policy_future: BoxFuture<
            'static,
            (
                ResponseCachePolicy<Res>,
                Option<Vec<hitbox_core::tag::CacheTag>>,
            ),
        >,
        permit: Option<OwnedSemaphorePermit>,
        ctx: BoxContext,
        cache_key: CacheKey,
    },
    /// Return response directly (non-cacheable path)
    Response(Response<Res>),
}

impl<Res> PollUpstreamTransition<Res>
where
    Res: CacheableResponse,
{
    pub fn into_state<'req, Req, U, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Req: CacheableRequest + 'req,
        U: Upstream<Req, Response = Res> + 'req,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            PollUpstreamTransition::CheckResponseCachePolicy {
                cache_policy_future,
                permit,
                ctx,
                cache_key,
            } => State::CheckResponseCachePolicy {
                cache_policy: cache_policy_future,
                state: Some(CheckResponseCachePolicy::new(
                    permit, ctx, cache_key, parent,
                )),
            },
            PollUpstreamTransition::Response(s) => {
                State::Response(Some(Response::new(s.response, s.ctx, parent)))
            }
        }
    }
}

impl<Res> std::fmt::Debug for PollUpstreamTransition<Res>
where
    Res: CacheableResponse,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckResponseCachePolicy { .. } => {
                f.write_str("PollUpstreamTransition::CheckResponseCachePolicy")
            }
            Self::Response(_) => f.write_str("PollUpstreamTransition::Response"),
        }
    }
}

// =============================================================================
// CheckResponseCachePolicyTransition
// =============================================================================

/// Transitions from CheckResponseCachePolicy state.
pub enum CheckResponseCachePolicyTransition<Res>
where
    Res: CacheableResponse,
{
    /// Response is cacheable - proceed to update cache
    UpdateCache {
        update_cache_future: UpdateCacheFuture<Res>,
    },
    /// Response is not cacheable - return directly
    Response(Response<Res>),
}

impl<Res> CheckResponseCachePolicyTransition<Res>
where
    Res: CacheableResponse,
{
    pub fn into_state<'req, Req, U, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Req: CacheableRequest + 'req,
        U: Upstream<Req, Response = Res> + 'req,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            CheckResponseCachePolicyTransition::UpdateCache {
                update_cache_future,
            } => State::UpdateCache {
                update_cache_future,
                state: Some(UpdateCache::new(parent)),
            },
            CheckResponseCachePolicyTransition::Response(s) => {
                State::Response(Some(Response::new(s.response, s.ctx, parent)))
            }
        }
    }
}

impl<Res> std::fmt::Debug for CheckResponseCachePolicyTransition<Res>
where
    Res: CacheableResponse,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpdateCache { .. } => {
                f.write_str("CheckResponseCachePolicyTransition::UpdateCache")
            }
            Self::Response(_) => f.write_str("CheckResponseCachePolicyTransition::Response"),
        }
    }
}

// =============================================================================
// UpdateCacheTransition
// =============================================================================

/// Transitions from UpdateCache state.
pub enum UpdateCacheTransition<Res> {
    Response(Response<Res>),
}

impl<Res> UpdateCacheTransition<Res> {
    pub fn into_state<'req, Req, U, ReqP, E, ReqTE>(
        self,
        parent: &Span,
    ) -> State<'req, Res, Req, U, ReqP, E, ReqTE>
    where
        Res: CacheableResponse,
        Req: CacheableRequest + 'req,
        U: Upstream<Req, Response = Res> + 'req,
        ReqP: Predicate<Subject = Req>,
        E: Extractor<Subject = Req>,
        ReqTE: TagExtractor<Subject = Req>,
    {
        match self {
            UpdateCacheTransition::Response(s) => {
                State::Response(Some(Response::new(s.response, s.ctx, parent)))
            }
        }
    }
}

impl<Res> std::fmt::Debug for UpdateCacheTransition<Res> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Response(_) => f.write_str("UpdateCacheTransition::Response"),
        }
    }
}
