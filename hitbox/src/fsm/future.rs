use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{future::BoxFuture, ready};
use hitbox_backend::{CachePolicy, CacheState, CacheableResponse};
use pin_project::pin_project;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    backend::CacheBackend,
    cache::CacheableRequest,
    fsm::{states::StateProj, PollCache, State},
    predicates::Predicate,
    Cacheable,
};

const POLL_AFTER_READY_ERROR: &str = "CacheFuture can't be polled after finishing";

#[pin_project]
pub struct CacheFuture<U, /*B, */ Req, Res, F>
where
    F: Future<Output = Res> + Send,
{
    // backend: Arc<B>,
    request: Option<Req>,
    upstream: Option<U>,
    #[pin]
    upstream_future: Option<F>,
}

impl<U, Req, Res, F> CacheFuture<U, Req, Res, F>
where
    // U: FnMut(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
    U: FnMut(Req) -> F,
    F: Future<Output = Res> + Send,
    // B: CacheBackend,
    Req: CacheableRequest,
{
    pub fn new(request: Req, upstream: U) -> Self {
        CacheFuture {
            request: Some(request),
            // backend,
            upstream: Some(upstream),
            upstream_future: None,
        }
    }
}

impl<U, Req, Res, F> Future for CacheFuture<U, Req, Res, F>
where
    U: FnMut(Req) -> F,
    F: Future<Output = Res> + Send,
    // U: FnMut(Req) -> Pin<Box<dyn Future<Output = Res> + Send>>,
    // B: CacheBackend,
    Req: CacheableRequest,
{
    type Output = Res;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if this.upstream_future.is_none() {
            let req = this.request.take().unwrap();
            let upstream_future = (this.upstream.take().unwrap())(req);
            this.upstream_future.set(Some(upstream_future));
        }
        Poll::Ready(ready!(this
            .upstream_future
            .as_pin_mut()
            .take()
            .unwrap()
            .poll(cx)))
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, time::Duration};

    use super::*;

    use async_trait::async_trait;
    use futures::FutureExt;
    use hitbox_backend::CachePolicy;

    use crate::{
        cache::{CacheKey, CacheableRequest},
        predicates::Predicate,
    };

    #[tokio::test]
    pub async fn test_cache_future() {
        pub struct Req {}
        pub struct CacheableReq {}

        impl CacheableReq {
            pub fn from_req(req: Req) -> Self {
                Self {}
            }

            pub fn into_req(self) -> Req {
                Req {}
            }
        }

        #[async_trait]
        impl CacheableRequest for CacheableReq {
            async fn cache_policy<P>(self, predicates: &[P]) -> crate::cache::CachePolicy<Self>
            where
                P: Predicate<Self> + Send + Sync,
            {
                crate::cache::CachePolicy::Cacheable(self)
            }
        }

        pub struct Res {}
        #[derive(Clone)]
        pub struct CacheableRes {}

        impl CacheableRes {
            pub fn from_res(res: Res) -> Self {
                Self {}
            }
            pub fn into_res(self) -> Res {
                Res {}
            }
        }

        #[async_trait]
        impl CacheableResponse for CacheableRes {
            type Cached = CacheableRes;

            async fn into_cached(self) -> Self::Cached {
                self
            }

            async fn from_cached(cached: Self::Cached) -> Self {
                cached
            }
        }

        #[derive(Clone)]
        pub struct Service {
            counter: u32,
        }

        impl Service {
            pub fn new() -> Self {
                Self { counter: 0 }
            }

            async fn call(&mut self, req: Req) -> Res {
                self.counter += 1;
                tokio::time::sleep(Duration::from_secs(3)).await;
                Res {}
            }
        }

        #[pin_project]
        pub struct UpstreamFuture {
            inner_future: BoxFuture<'static, CacheableRes>,
        }

        impl UpstreamFuture {
            pub fn new(inner: &Service, req: CacheableReq) -> Self {
                let mut inner_service = inner.clone();
                let f = Box::pin(async move {
                    inner_service
                        .call(req.into_req())
                        .map(CacheableRes::from_res)
                        .await
                });
                UpstreamFuture { inner_future: f }
            }
        }

        impl Future for UpstreamFuture {
            type Output = CacheableRes;
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.project();
                this.inner_future.as_mut().poll(cx)
            }
        }

        let req = CacheableReq {};
        let service = Service::new();
        // let upstream = move |req| {
        //     let mut s = service.clone();
        //     Box::pin(s.call(req).map(|res| Res {})) as Pin<Box<dyn Future<Output = Res> + Send>>
        // };
        // let fsm = CacheFuture::new(req, upstream);

        let upstream = |req| UpstreamFuture::new(&service, req);
        let fsm = CacheFuture::new(req, upstream);
        fsm.await;
    }
}

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
//

#[pin_project]
pub struct CacheFuture2<U, B, C, R>
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

impl<U, B, C, R> CacheFuture2<U, B, C, R>
where
    B: CacheBackend,
    U: Future,
    C: CacheableResponse,
    R: Cacheable,
{
    pub fn new(upstream: U, backend: Arc<B>, request: R) -> Self {
        CacheFuture2 {
            upstream,
            backend,
            request,
            state: State::Initial,
            poll_cache: None,
        }
    }
}
/*
impl<U, B, C, R> Future for CacheFuture2<U, B, C, R>
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
}*/
