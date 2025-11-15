use std::fmt::Debug;
use std::sync::Arc;

use futures::future::BoxFuture;
use hitbox_backend::BackendError;
use hitbox_core::{RequestCachePolicy, ResponseCachePolicy};
use pin_project::pin_project;
use tokio::sync::{broadcast, OwnedSemaphorePermit};

use crate::{CacheKey, CacheState, CacheValue, CacheableResponse, lock_manager::LockError};

pub type CacheResult<T> = Result<Option<CacheValue<T>>, BackendError>;
pub type PollCacheFuture<T> = BoxFuture<'static, CacheResult<T>>;
pub type UpdateCache<T> = BoxFuture<'static, (Result<(), BackendError>, T)>;
pub type RequestCachePolicyFuture<T> = BoxFuture<'static, RequestCachePolicy<T>>;
pub type CacheStateFuture<T> = BoxFuture<'static, CacheState<T>>;
pub type UpstreamFuture<T> = BoxFuture<'static, T>;
pub type LockFuture = BoxFuture<'static, Result<OwnedSemaphorePermit, LockError>>;
pub type BroadcastFuture<T> = BoxFuture<'static, Result<Arc<T>, broadcast::error::RecvError>>;

#[allow(missing_docs)]
#[pin_project(project = StateProj)]
pub enum State<Res, Req>
where
    Res: CacheableResponse,
{
    Initial,
    CheckRequestCachePolicy {
        #[pin]
        cache_policy_future: RequestCachePolicyFuture<Req>,
    },
    PollCache {
        #[pin]
        poll_cache: PollCacheFuture<Res::Cached>,
        request: Option<Req>,
    },
    // CachePolled {
    //     cache_result: CacheResult<C::Cached>,
    // },
    CheckCacheState {
        cache_state: CacheStateFuture<Res>,
        request: Option<Req>,
    },
    TryAcquireLock {
        key: CacheKey,
        request: Option<Req>,
        concurrency: usize,
    },
    WaitForLock {
        #[pin]
        lock_future: LockFuture,
        key: CacheKey,
        request: Option<Req>,
    },
    CheckCacheAfterWait {
        #[pin]
        cache_check: PollCacheFuture<Res::Cached>,
        permit: Option<OwnedSemaphorePermit>,
        request: Option<Req>,
    },
    WaitForBroadcast {
        #[pin]
        broadcast_future: BroadcastFuture<Res::Cached>,
        key: CacheKey,
    },
    CheckCacheAfterBroadcastFailure {
        #[pin]
        cache_check: PollCacheFuture<Res::Cached>,
    },
    ConvertCachedToResponse {
        #[pin]
        response_future: BoxFuture<'static, Res>,
    },
    PollUpstream {
        upstream_future: UpstreamFuture<Res>,
    },
    UpstreamPolled {
        upstream_result: Option<Res>,
    },
    CheckResponseCachePolicy {
        #[pin]
        cache_policy: BoxFuture<'static, ResponseCachePolicy<Res>>,
    },
    UpdateCache {
        #[pin]
        update_cache_future: UpdateCache<Res>,
    },
    Response {
        response: Option<Res>,
    },
}

impl<Res, Req> Debug for State<Res, Req>
where
    Res: CacheableResponse,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Initial => f.write_str("State::Initial"),
            State::CheckRequestCachePolicy { .. } => f.write_str("State::CheckRequestCachePolicy"),
            State::PollCache { .. } => f.write_str("State::PollCache"),
            // State::CachePolled { .. } => f.write_str("State::PollCache"),
            State::CheckCacheState { .. } => f.write_str("State::CheckCacheState"),
            State::TryAcquireLock { .. } => f.write_str("State::TryAcquireLock"),
            State::WaitForLock { .. } => f.write_str("State::WaitForLock"),
            State::CheckCacheAfterWait { .. } => f.write_str("State::CheckCacheAfterWait"),
            State::WaitForBroadcast { .. } => f.write_str("State::WaitForBroadcast"),
            State::CheckCacheAfterBroadcastFailure { .. } => f.write_str("State::CheckCacheAfterBroadcastFailure"),
            State::ConvertCachedToResponse { .. } => f.write_str("State::ConvertCachedToResponse"),
            State::CheckResponseCachePolicy { .. } => {
                f.write_str("State::CheckResponseCachePolicy")
            }
            State::PollUpstream { .. } => f.write_str("State::PollUpstream"),
            State::UpstreamPolled { .. } => f.write_str("State::UpstreamPolled"),
            State::UpdateCache { .. } => f.write_str("State::UpdateCache"),
            State::Response { .. } => f.write_str("State::Response"),
        }
    }
}
