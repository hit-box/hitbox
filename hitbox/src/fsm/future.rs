use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::ready;
use hitbox_backend::{CachePolicy, CacheState, CacheableResponse, CacheableResponseWrapper};
use pin_project::pin_project;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    backend::CacheBackend,
    fsm::{states::StateProj, PollCache, State},
    Cacheable,
};

const POLL_AFTER_READY_ERROR: &str = "CacheFuture can't be polled after finishing";

#[pin_project]
pub struct CacheFuture<U, B, C, R>
where
    U: Future,
    B: CacheBackend,
    C: CacheableResponse,
    R: Cacheable,
{
    #[pin]
    upstream: U,
    backend: Arc<B>,
    request: R,
    #[pin]
    state: State<U::Output, C>,
    #[pin]
    poll_cache: Option<PollCache<C>>,
}

impl<U, B, C, R> CacheFuture<U, B, C, R>
where
    B: CacheBackend,
    U: Future,
    C: CacheableResponse,
    R: Cacheable,
{
    pub fn new(upstream: U, backend: Arc<B>, request: R) -> Self {
        CacheFuture {
            upstream,
            backend,
            request,
            state: State::Initial,
            poll_cache: None,
        }
    }
}

impl<U, B, C, R> Future for CacheFuture<U, B, C, R>
where
    B: CacheBackend + Send + Sync + 'static,
    U: Future + Send,
    C: CacheableResponseWrapper<Source = U::Output> + CacheableResponse + Send + 'static,
    C::Cached: Send + DeserializeOwned + Serialize + Debug + Clone,
    R: Cacheable,
{
    type Output = U::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            dbg!(&this.state);
            let state = match this.state.as_mut().project() {
                StateProj::Initial => {
                    let backend = this.backend.clone();
                    // let cache_key = this.cache_key.clone();
                    let cache_key = "fake::key".to_owned();
                    let poll_cache = Box::pin(async move { backend.get::<C>(cache_key).await });
                    // this.poll_cache.set(Some(poll_cache));
                    State::PollCache { poll_cache }
                }
                StateProj::PollCache { poll_cache } => {
                    let cached = ready!(poll_cache.poll(cx)).unwrap();
                    dbg!(&cached);
                    match cached {
                        Some(cached_value) => match C::from_cached(cached_value) {
                            CacheState::Actual(value) => State::Response {
                                response: Some(value),
                            },
                            _ => State::PollUpstream,
                        },
                        None => State::PollUpstream,
                    }
                }
                StateProj::PollUpstream => {
                    let res = ready!(this.upstream.as_mut().poll(cx));
                    State::UpstreamPolled {
                        upstream_result: Some(res),
                    }
                }
                StateProj::UpstreamPolled { upstream_result } => {
                    let upstream_result = upstream_result.take().expect(POLL_AFTER_READY_ERROR);
                    let cacheable = C::from_source(upstream_result);
                    let cache_policy = Box::pin(async move { cacheable.cache_policy().await });
                    State::CheckResponseCachePolicy { cache_policy }
                }
                StateProj::CheckResponseCachePolicy { cache_policy } => {
                    let cached_value = match ready!(cache_policy.poll(cx)) {
                        CachePolicy::Cacheable(cached_value) => cached_value,
                        _ => unimplemented!(),
                    };
                    let backend = this.backend.clone();
                    // let cache_key = this.cache_key.clone();
                    let cache_key = "fake::key".to_owned();
                    let cached = cached_value.clone();
                    let update_cache =
                        Box::pin(async move { backend.set::<C>(cache_key, cached, None).await });
                    let cacheable = match C::from_cached(cached_value) {
                        CacheState::Actual(cacheable) => cacheable,
                        _ => unimplemented!(),
                    };
                    State::UpdateCache {
                        update_cache,
                        upstream_result: Some(cacheable),
                    }
                }
                StateProj::UpdateCache {
                    update_cache,
                    upstream_result,
                } => {
                    ready!(update_cache.poll(cx));
                    State::Response {
                        response: Some(upstream_result.take().expect(POLL_AFTER_READY_ERROR)),
                    }
                }
                StateProj::Response { response } => {
                    let response = response.take().expect(POLL_AFTER_READY_ERROR);
                    return Poll::Ready(response.into_source());
                }
                _ => unimplemented!(),
            };
            this.state.set(state);
        }
    }
}
