use std::fmt::Debug;

use futures::future::BoxFuture;
use hitbox_backend::BackendError;
use hitbox_core::{RequestCachePolicy, ResponseCachePolicy};
use pin_project::pin_project;

use crate::{CacheState, CacheableResponse, CachedValue};

pub type CacheResult<R> = Result<Option<CachedValue<R>>, BackendError>;
pub type PollCache<R> = BoxFuture<'static, CacheResult<R>>;
pub type UpdateCache<R> = BoxFuture<'static, (Result<(), BackendError>, R)>;

#[allow(missing_docs)]
#[pin_project(project = StateProj)]
pub enum State<U, C, R>
where
    C: CacheableResponse,
{
    Initial,
    CheckRequestCachePolicy {
        #[pin]
        cache_policy_future: BoxFuture<'static, RequestCachePolicy<R>>,
    },
    PollCache {
        #[pin]
        poll_cache: PollCache<C::Cached>,
        request: Option<R>,
    },
    // CachePolled {
    //     cache_result: CacheResult<C::Cached>,
    // },
    CheckCacheState {
        cache_state: BoxFuture<'static, CacheState<C>>,
    },
    PollUpstream {
        upstream_future: BoxFuture<'static, C>,
    },
    UpstreamPolled {
        upstream_result: Option<U>,
    },
    CheckResponseCachePolicy {
        #[pin]
        cache_policy: BoxFuture<'static, ResponseCachePolicy<C>>,
    },
    UpdateCache {
        #[pin]
        update_cache_future: UpdateCache<C>,
    },
    Response {
        response: Option<C>,
    },
}

impl<U, C, R> Debug for State<U, C, R>
where
    C: CacheableResponse,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Initial => f.write_str("State::Initial"),
            State::CheckRequestCachePolicy { .. } => f.write_str("State::CheckRequestCachePolicy"),
            State::PollCache { .. } => f.write_str("State::PollCache"),
            // State::CachePolled { .. } => f.write_str("State::PollCache"),
            State::CheckCacheState { .. } => f.write_str("State::CheckCacheState"),
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
