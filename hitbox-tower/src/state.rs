use std::{
    fmt::{Debug, Write},
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{future::BoxFuture, ready, Future, FutureExt};
use hitbox::{
    dev::{BackendError, CacheBackend},
    CachedValue,
};
use hitbox_backend::{
    response2::{CachePolicy, CacheableResponse, CacheableWrapper},
    Backend,
};
use http::{Request, Response};
use pin_project::pin_project;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tower::Service;

// pub type CacheResult<R, E> =
// Result<Option<CachedValue<hitbox_http::CacheableResponse<R, E>>>, BackendError>;
// pub type PollCache<R, E> = BoxFuture<'static, CacheResult<R, E>>;
// pub type PollCache2<R, E> = Pin<Box<dyn Future<Output = CacheResult<R, E>> + Send>>;

#[pin_project(project = CacheStateProj)]
pub enum CacheState<PU, Res, E, Req, PC>
where
    PC: 'static,
{
    Initial {
        req: Option<Request<Req>>,
    },
    PollCache {
        // #[pin]
        poll_cache: PC,
    },
    CachePolled,
    PollUpstream {
        #[pin]
        poll_upstream: PU,
    },
    UpstreamPolled,
    Response {
        #[pin]
        response: Result<Res, E>,
    },
}

impl<PU, Res, E, Req, PC> Debug for CacheState<PU, Res, E, Req, PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheState::Initial { req: _ } => f.write_str("State::Initial"),
            CacheState::PollCache { poll_cache: _ } => f.write_str("State::PollCache"),
            CacheState::CachePolled => f.write_str("State::CachePolled"),
            CacheState::PollUpstream { poll_upstream: _ } => f.write_str("State::PollUpstream"),
            CacheState::UpstreamPolled => f.write_str("State::UpstreamPolled"),
            CacheState::Response { response: _ } => f.write_str("State::Response"),
        }
    }
}

const POLL_AFTER_READY_ERROR: &str = "CacheFuture can't be polled after finishing";

pub type CacheResult<R> = Result<Option<CachedValue<R>>, BackendError>;
pub type PollCache<R> = BoxFuture<'static, CacheResult<R>>;
pub type UpdateCache = BoxFuture<'static, Result<(), BackendError>>;

#[pin_project(project = StateProj)]
enum State<U, C>
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

#[pin_project]
pub struct CacheFuture<U, B, C>
where
    U: Future,
    B: CacheBackend,
    // U::Output: CacheableResponse<Cached = C>,
    // U::Output: IntoCacheable,
    C: CacheableResponse,
{
    #[pin]
    upstream: U,
    backend: Arc<B>,
    cache_key: String,
    #[pin]
    state: State<U::Output, C>,

    #[pin]
    poll_cache: Option<PollCache<C>>,
    // poll_cache: Option<
    // Pin<Box<dyn Future<Output = CacheResult<<U::Output as IntoCacheable>::Cacheable>> + Send>>,
    // >,
}

impl<U, B, C> CacheFuture<U, B, C>
where
    B: CacheBackend,
    U: Future,
    // <U::Output as CacheableResponse>::Cached: DeserializeOwned,
    C: CacheableResponse,
{
    pub fn new(upstream: U, backend: Arc<B>, cache_key: String) -> Self {
        CacheFuture {
            upstream,
            backend,
            cache_key,
            state: State::Initial,
            poll_cache: None,
        }
    }
}

impl<U, B, C> Future for CacheFuture<U, B, C>
where
    B: CacheBackend + Send + Sync + 'static,
    U: Future + Send,
    C: CacheableWrapper<Source = U::Output> + CacheableResponse + Send + 'static,
    C::Cached: Send + DeserializeOwned + Serialize + Debug + Clone,
    // U::Output: From<C>,
    // C: CacheableResponse + Send + 'static,
    // C::Cached: DeserializeOwned,
    // C: From<U::Output> + CacheableResponse + Send + 'static + From<C::Cached>,
{
    type Output = U::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            dbg!(&this.state);
            let state = match this.state.as_mut().project() {
                StateProj::Initial => {
                    let backend = this.backend.clone();
                    let cache_key = this.cache_key.clone();
                    let poll_cache = Box::pin(async move { backend.get::<C>(cache_key).await });
                    // this.poll_cache.set(Some(poll_cache));
                    State::PollCache { poll_cache }
                }
                StateProj::PollCache { poll_cache } => {
                    let cached = ready!(poll_cache.poll(cx)).unwrap();
                    dbg!(&cached);
                    match cached {
                        Some(cached_value) => match C::from_cached(cached_value) {
                            hitbox_backend::response2::CacheState::Actual(value) => {
                                State::Response {
                                    response: Some(value),
                                }
                            }
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
                    State::CheckCachePolicy { cache_policy }
                }
                StateProj::CheckCachePolicy { cache_policy } => {
                    let cached_value = match ready!(cache_policy.poll(cx)) {
                        CachePolicy::Cacheable(cached_value) => cached_value,
                        _ => unimplemented!(),
                    };
                    let backend = this.backend.clone();
                    let cache_key = this.cache_key.clone();
                    let cached = cached_value.clone();
                    let update_cache =
                        Box::pin(async move { backend.set::<C>(cache_key, cached, None).await });
                    let cacheable = match C::from_cached(cached_value) {
                        hitbox_backend::response2::CacheState::Actual(cacheable) => cacheable,
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

// pub trait IntoCacheable {
//     type Cacheable: CacheableResponse;
//     fn to_cacheable(&self) -> CachedValue<Self::Cacheable>;
//     fn from_cacheable(cached: Self::Cacheable) -> Self;
// }
//
// impl<T, E> IntoCacheable for Result<Response<T>, E> {
//     type Cacheable = Result<hitbox_http::CacheableResponse, E>;
//     fn from_cacheable(cached: Self::Cacheable) -> Self {
//         unimplemented!()
//     }
//
//     fn to_cacheable(&self) -> CachedValue<Self::Cacheable> {
//         let cacheable = self.map(hitbox_http::CacheableResponse::from_response);
//     }
// }
//
//
//
//
//
//
//
//
//
//
//
// =========================================================================

// impl<R, E> IntoCacheable for Result<Response<R>, E>
// where
// E: Debug,
// {
// type Cacheable = hitbox_http::CacheableResponse<R, E>;

// fn into_cacheable(self) -> Self::Cacheable {
// hitbox_http::CacheableResponse::from_response(self)
// }

// fn from_cacheable(cached: Self::Cacheable) -> Self {
// cached.into_response()
// }
// }

// #[pin_project]
// pub struct CacheFutureWrapper<U, B, E, R>
// where
// B: CacheBackend,
// U: Future<Output = hitbox_http::CacheableResponse<R, E>>,
// E: Debug,
// {
// #[pin]
// inner: CacheFuture<U, B>,
// }

// impl<U, B, E, R> CacheFutureWrapper<U, B, E, R>
// where
// B: CacheBackend,
// U: Future<Output = hitbox_http::CacheableResponse<R, E>>,
// E: Debug,
// {
// pub fn new<C>(upstream: C, backend: B, cache_key: String) -> impl Future
// where
// C: Future<Output = Result<Response<R>, E>>,
// E: Debug,
// {
// let upstream = upstream.map(hitbox_http::CacheableResponse::from_response);
// let inner = CacheFuture::new(upstream, backend, cache_key);
// CacheFutureWrapper {
// inner
// }
// }
// }
//
// #[pin_project]
// pub struct CacheFutureWrapper<I>
// where
// I: Future,
// {
// #[pin]
// inner: I,
// }

// impl<I> CacheFutureWrapper<I>
// where
// I: Future,
// {
// pub fn new<C, B>(upstream: C, backend: B, cache_key: String) -> impl Future
// where
// C: Future<Output = Result<Response<R>, E>>,
// E: Debug,
// {
// let upstream = upstream.map(hitbox_http::CacheableResponse::from_response);
// let inner = CacheFuture::new(upstream, backend, cache_key);
// CacheFutureWrapper {
// inner
// }
// }
// }

// impl<U, B, E, R> Future for CacheFutureWrapper<U, B, E, R>
// where
// B: CacheBackend,
// U: Future<Output = hitbox_http::CacheableResponse<R, E>>,
// E: Debug,
// {
// type Output = Result<Response<R>, E>;
// fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
// let mut this = self.project();
// let res = ready!(this.inner.as_mut().poll(cx));
// Poll::Ready(res.into_response())
// }
// }

// pub fn new<U, B, E, R>(upstream: U, backend: B, cache_key: String) -> impl Future
// where
// U: Future<Output = Result<Response<R>, E>>,
// E: Debug,
// {
// let upstream = upstream.map(hitbox_http::CacheableResponse::from_response);
// let inner = CacheFuture::new(upstream, backend, cache_key);
// CacheFutureWrapper {
// inner
// }
// }

// pub struct Transformer<F> {
// inner: F
// }

// pub trait Transform<Body, E>
// where
// Self: Future
// {
// type Transformer: Future;

// fn transform(self) -> Self::Output;
// }

// impl<Body, E, F> Transform<Body, E> for F
// where
// F: Future<Output = Result<Response<Body>, E>>,
// {
// type Transformer = BoxFuture<'static, hitbox_http::CacheableResponse<Body, E>>;

// fn transform(self) -> Self::Transformer {
// Box::pin(self.then(hitbox_http::CacheableResponse::from_response))
// }
// }
