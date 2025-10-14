use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use crate::{policy::PolicyConfig, CachePolicy, CacheState, CacheStatus, CacheableResponse};
use futures::ready;
use hitbox_core::{CacheablePolicyData, EntityPolicyConfig};
use pin_project::pin_project;
use serde::{de::DeserializeOwned, Serialize};
use tracing::debug;

use crate::{
    backend::CacheBackend,
    fsm::{states::StateProj, PollCacheFuture, State},
    CacheKey, CacheableRequest, Extractor, Predicate,
};

const POLL_AFTER_READY_ERROR: &str = "CacheFuture can't be polled after finishing";

// #[cfg(test)]
// mod tests {
//     use std::{convert::Infallible, time::Duration};
//
//     use super::*;
//
//     use async_trait::async_trait;
//     use futures::FutureExt;
//     use hitbox_backend::CachePolicy;
//
//     use crate::{
//         cache::{CacheKey, CacheableRequest},
//         predicates::Predicate,
//     };
//
//     #[tokio::test]
//     pub async fn test_cache_future() {
//         pub struct Req {}
//         pub struct CacheableReq {}
//
//         impl CacheableReq {
//             pub fn from_req(req: Req) -> Self {
//                 Self {}
//             }
//
//             pub fn into_req(self) -> Req {
//                 Req {}
//             }
//         }
//
//         #[async_trait]
//         impl CacheableRequest for CacheableReq {
//             async fn cache_policy(
//                 self,
//                 predicates: &[Box<dyn Predicate<Self> + Send>],
//             ) -> crate::cache::CachePolicy<Self> {
//                 crate::cache::CachePolicy::Cacheable(self)
//             }
//         }
//
//         pub struct Res {}
//         #[derive(Clone)]
//         pub struct CacheableRes {}
//
//         impl CacheableRes {
//             pub fn from_res(res: Res) -> Self {
//                 Self {}
//             }
//             pub fn into_res(self) -> Res {
//                 Res {}
//             }
//         }
//
//         #[async_trait]
//         impl CacheableResponse for CacheableRes {
//             type Cached = CacheableRes;
//
//             async fn into_cached(self) -> Self::Cached {
//                 self
//             }
//
//             async fn from_cached(cached: Self::Cached) -> Self {
//                 cached
//             }
//         }
//
//         #[derive(Clone)]
//         pub struct Service {
//             counter: u32,
//         }
//
//         impl Service {
//             pub fn new() -> Self {
//                 Self { counter: 0 }
//             }
//
//             async fn call(&mut self, req: Req) -> Res {
//                 self.counter += 1;
//                 tokio::time::sleep(Duration::from_secs(3)).await;
//                 Res {}
//             }
//         }
//
//         #[pin_project]
//         pub struct UpstreamFuture {
//             inner_future: BoxFuture<'static, CacheableRes>,
//         }
//
//         impl UpstreamFuture {
//             pub fn new(inner: &Service, req: CacheableReq) -> Self {
//                 let mut inner_service = inner.clone();
//                 let f = Box::pin(async move {
//                     inner_service
//                         .call(req.into_req())
//                         .map(CacheableRes::from_res)
//                         .await
//                 });
//                 UpstreamFuture { inner_future: f }
//             }
//         }
//
//         impl Future for UpstreamFuture {
//             type Output = CacheableRes;
//             fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//                 let this = self.project();
//                 this.inner_future.as_mut().poll(cx)
//             }
//         }
//
//         let req = CacheableReq {};
//         let service = Service::new();
//         // let upstream = move |req| {
//         //     let mut s = service.clone();
//         //     Box::pin(s.call(req).map(|res| Res {})) as Pin<Box<dyn Future<Output = Res> + Send>>
//         // };
//         // let fsm = CacheFuture::new(req, upstream);
//
//         let upstream = |req| UpstreamFuture::new(&service, req);
//         let fsm = CacheFuture3::new(req, upstream);
//         fsm.await;
//     }
// }

pub trait Transform<Req, Res> {
    type Future;
    type Response;

    fn upstream_transform(&self, req: Req) -> Self::Future;
    fn response_transform(
        &self,
        res: Res,
        cache_status: Option<crate::CacheStatus>,
    ) -> Self::Response;
}

#[pin_project]
pub struct CacheFuture<B, Req, Res, T>
where
    T: Transform<Req, Res>,
    T::Future: Future<Output = Res> + Send,
    B: CacheBackend,
    Res: CacheableResponse,
    Req: CacheableRequest,
{
    transformer: T,
    backend: Arc<B>,
    request: Option<Req>,
    cache_key: Option<CacheKey>,
    cache_status: crate::CacheStatus,
    cache_enabled: bool,
    #[pin]
    state: State<Res, Req>,
    #[pin]
    poll_cache: Option<PollCacheFuture<Res>>,
    request_predicates: Arc<dyn Predicate<Subject = Req> + Send + Sync>,
    response_predicates: Arc<dyn Predicate<Subject = Res::Subject> + Send + Sync>,
    key_extractors: Arc<dyn Extractor<Subject = Req> + Send + Sync>,
    policy: Arc<crate::policy::PolicyConfig>,
}

impl<B, Req, Res, T> CacheFuture<B, Req, Res, T>
where
    T: Transform<Req, Res>,
    T::Future: Future<Output = Res> + Send,
    B: CacheBackend,
    Res: CacheableResponse,
    Req: CacheableRequest,
{
    pub fn new(
        backend: Arc<B>,
        request: Req,
        transformer: T,
        request_predicates: Arc<dyn Predicate<Subject = Req> + Send + Sync>,
        response_predicates: Arc<dyn Predicate<Subject = Res::Subject> + Send + Sync>,
        key_extractors: Arc<dyn Extractor<Subject = Req> + Send + Sync>,
        policy: Arc<crate::policy::PolicyConfig>,
    ) -> Self {
        let cache_enabled = matches!(policy.as_ref(), crate::policy::PolicyConfig::Enabled(_));
        CacheFuture {
            transformer,
            backend,
            cache_key: None,
            cache_status: crate::CacheStatus::Miss,
            cache_enabled,
            request: Some(request),
            state: State::Initial,
            poll_cache: None,
            request_predicates,
            response_predicates,
            key_extractors,
            policy,
        }
    }
}

impl<B, Req, Res, T> Future for CacheFuture<B, Req, Res, T>
where
    T: Transform<Req, Res>,
    T::Future: Future<Output = Res> + Send + 'static,
    B: CacheBackend + Send + Sync + 'static,
    Res: CacheableResponse,
    Res::Cached: Serialize + DeserializeOwned + Send + Sync,
    Req: CacheableRequest + Send + 'static,

    // Debug bounds
    Req: Debug,
    Res::Cached: Debug,
{
    type Output = T::Response;

    // #[instrument(skip(self, cx), fields(state = ?self.state, request = type_name::<T::Response>(), backend = type_name::<B>()))]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            let state = match this.state.as_mut().project() {
                StateProj::Initial => {
                    let predicates = this.request_predicates.clone();
                    let extractors = this.key_extractors.clone();
                    let request = this.request.take().expect(POLL_AFTER_READY_ERROR);
                    match this.policy.as_ref() {
                        PolicyConfig::Enabled(_) => {
                            let cache_policy_future = Box::pin(async move {
                                request.cache_policy(predicates, extractors).await
                            });
                            State::CheckRequestCachePolicy {
                                cache_policy_future,
                            }
                        }
                        PolicyConfig::Disabled => {
                            let upstream_future =
                                Box::pin(this.transformer.upstream_transform(request));
                            State::PollUpstream { upstream_future }
                        }
                    }
                }
                StateProj::CheckRequestCachePolicy {
                    cache_policy_future,
                } => {
                    let policy = ready!(cache_policy_future.poll(cx));
                    match policy {
                        CachePolicy::Cacheable(CacheablePolicyData { key, request }) => {
                            let backend = this.backend.clone();
                            let cache_key = key.clone();
                            let _ = this.cache_key.insert(key);
                            let poll_cache =
                                Box::pin(async move { backend.get::<Res>(&cache_key).await });
                            State::PollCache {
                                poll_cache,
                                request: Some(request),
                            }
                        }
                        CachePolicy::NonCacheable(request) => {
                            let upstream_future =
                                Box::pin(this.transformer.upstream_transform(request));
                            State::PollUpstream { upstream_future }
                        }
                    }
                }
                StateProj::PollCache {
                    poll_cache,
                    request,
                } => {
                    let cached = ready!(poll_cache.poll(cx)).unwrap_or_else(|_err| {
                        //println!("cache backend error: {err}");
                        None
                    });
                    match cached {
                        Some(cached_value) => State::CheckCacheState {
                            cache_state: Box::pin(cached_value.cache_state()),
                            request: request.take(),
                        },
                        None => {
                            let upstream_future =
                                Box::pin(this.transformer.upstream_transform(
                                    request.take().expect(POLL_AFTER_READY_ERROR),
                                ));
                            State::PollUpstream { upstream_future }
                        }
                    }
                }
                StateProj::CheckCacheState {
                    cache_state,
                    request,
                } => {
                    let state = ready!(cache_state.as_mut().poll(cx));
                    *this.cache_status = CacheStatus::Hit;
                    match state {
                        CacheState::Actual(response) => State::Response {
                            response: Some(response),
                        },
                        CacheState::Stale(response) => State::Response {
                            response: Some(response),
                        },
                        // TODO: remove code duplication with PollCache (upstream_future creation)
                        CacheState::Expired(_response) => {
                            *this.cache_status = CacheStatus::Miss;
                            let upstream_future =
                                Box::pin(this.transformer.upstream_transform(
                                    request.take().expect(POLL_AFTER_READY_ERROR),
                                ));
                            State::PollUpstream { upstream_future }
                        }
                    }
                }
                StateProj::PollUpstream { upstream_future } => {
                    let res = ready!(upstream_future.as_mut().poll(cx));
                    State::UpstreamPolled {
                        upstream_result: Some(res),
                    }
                }
                StateProj::UpstreamPolled { upstream_result } => {
                    let upstream_result = upstream_result.take().expect(POLL_AFTER_READY_ERROR);
                    let predicates = this.response_predicates.clone();
                    match this.cache_key {
                        Some(_cache_key) => {
                            let entity_config = match this.policy.as_ref() {
                                PolicyConfig::Enabled(config) => EntityPolicyConfig {
                                    ttl: config.ttl.map(|s| Duration::from_secs(s as u64)),
                                    stale_ttl: config.stale.map(|s| Duration::from_secs(s as u64)),
                                },
                                PolicyConfig::Disabled => EntityPolicyConfig::default(),
                            };
                            State::CheckResponseCachePolicy {
                                cache_policy: Box::pin(async move {
                                    upstream_result
                                        .cache_policy(predicates, &entity_config)
                                        .await
                                }),
                            }
                        }
                        None => State::Response {
                            response: Some(upstream_result),
                        },
                    }
                }
                StateProj::CheckResponseCachePolicy { cache_policy } => {
                    let policy = ready!(cache_policy.poll(cx));
                    let backend = this.backend.clone();
                    let cache_key = this.cache_key.take().expect("CacheKey not found");
                    match policy {
                        CachePolicy::Cacheable(cache_value) => {
                            let update_cache_future = Box::pin(async move {
                                let update_cache_result =
                                    backend.set::<Res>(&cache_key, &cache_value, None).await;
                                let upstream_result =
                                    Res::from_cached(cache_value.into_inner()).await;
                                (update_cache_result, upstream_result)
                            });
                            State::UpdateCache {
                                update_cache_future,
                            }
                        }
                        CachePolicy::NonCacheable(response) => State::Response {
                            response: Some(response),
                        },
                    }
                }
                StateProj::UpdateCache {
                    update_cache_future,
                } => {
                    // TODO: check backend result
                    let (_backend_result, upstream_result) = ready!(update_cache_future.poll(cx));
                    State::Response {
                        response: Some(upstream_result),
                    }
                }
                StateProj::Response { response } => {
                    let response = this.transformer.response_transform(
                        response.take().expect(POLL_AFTER_READY_ERROR),
                        if *this.cache_enabled {
                            Some(*this.cache_status)
                        } else {
                            None
                        },
                    );
                    return Poll::Ready(response);
                }
            };
            debug!("{:?}", &state);
            this.state.set(state);
        }
    }
}
