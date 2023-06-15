use std::fmt::Debug;

use futures::future::BoxFuture;
use hitbox_backend::{BackendError, CachePolicy, CacheableResponse, CachedValue};
use pin_project::pin_project;

pub type CacheResult<R> = Result<Option<CachedValue<R>>, BackendError>;
pub type PollCache<R> = BoxFuture<'static, CacheResult<R>>;
pub type UpdateCache = BoxFuture<'static, Result<(), BackendError>>;

#[pin_project(project = StateProj)]
pub enum State<U, C>
where
    C: CacheableResponse,
{
    Initial,
    PollCache {
        #[pin]
        poll_cache: PollCache<C::Cached>,
    },
    CachePolled {
        cache_result: CacheResult<C::Cached>,
    },
    PollUpstream,
    UpstreamPolled {
        upstream_result: Option<U>,
    },
    CheckCachePolicy {
        #[pin]
        cache_policy: BoxFuture<'static, CachePolicy<CachedValue<C::Cached>>>,
    },
    UpdateCache {
        #[pin]
        update_cache: UpdateCache,
        upstream_result: Option<C>,
    },
    Response {
        response: Option<C>,
    },
}

impl<U, C> Debug for State<U, C>
where
    C: CacheableResponse,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Initial => f.write_str("State::Initial"),
            State::PollCache { .. } => f.write_str("State::PollCache"),
            State::CachePolled { .. } => f.write_str("State::PollCache"),
            State::CheckCachePolicy { .. } => f.write_str("State::CheckCachePolicy"),
            State::PollUpstream { .. } => f.write_str("State::PollUpstream"),
            State::UpstreamPolled { .. } => f.write_str("State::UpstreamPolled"),
            State::UpdateCache { .. } => f.write_str("State::UpdateCache"),
            State::Response { .. } => f.write_str("State::Response"),
        }
    }
}
