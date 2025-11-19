use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use crate::{CachePolicy, CacheState, CacheStatus, CacheableResponse, policy::PolicyConfig};
use futures::ready;
use hitbox_core::{CacheablePolicyData, EntityPolicyConfig, Upstream};
use pin_project::pin_project;
use serde::{Serialize, de::DeserializeOwned};
use tracing::debug;

use crate::{
    CacheKey, CacheableRequest, Extractor, Predicate,
    backend::CacheBackend,
    concurrency::{ConcurrencyDecision, ConcurrencyManager},
    fsm::{PollCacheFuture, State, states::StateProj},
};

const POLL_AFTER_READY_ERROR: &str = "CacheFuture can't be polled after finishing";

#[pin_project]
pub struct CacheFuture<B, Req, Res, U, C>
where
    U: Upstream<Req, Response = Res>,
    B: CacheBackend,
    Res: CacheableResponse,
    Req: CacheableRequest,
    C: ConcurrencyManager<Res>,
{
    upstream: U,
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
    concurrency_manager: Arc<C>,
}

impl<B, Req, Res, U, C> CacheFuture<B, Req, Res, U, C>
where
    U: Upstream<Req, Response = Res>,
    B: CacheBackend,
    Res: CacheableResponse,
    Req: CacheableRequest,
    C: ConcurrencyManager<Res>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        backend: Arc<B>,
        request: Req,
        upstream: U,
        request_predicates: Arc<dyn Predicate<Subject = Req> + Send + Sync>,
        response_predicates: Arc<dyn Predicate<Subject = Res::Subject> + Send + Sync>,
        key_extractors: Arc<dyn Extractor<Subject = Req> + Send + Sync>,
        policy: Arc<crate::policy::PolicyConfig>,
        concurrency_manager: Arc<C>,
    ) -> Self {
        let cache_enabled = matches!(policy.as_ref(), crate::policy::PolicyConfig::Enabled(_));
        CacheFuture {
            upstream,
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
            concurrency_manager,
        }
    }
}

impl<B, Req, Res, U, C> Future for CacheFuture<B, Req, Res, U, C>
where
    U: Upstream<Req, Response = Res>,
    U::Future: Send + 'static,
    B: CacheBackend + Send + Sync + 'static,
    Res: CacheableResponse,
    Res::Cached: Serialize + DeserializeOwned + Send + Sync,
    Req: CacheableRequest + Send + 'static,
    C: ConcurrencyManager<Res>,
    // Debug bounds
    Req: Debug,
    Res::Cached: Debug,
{
    type Output = (Res, crate::CacheContext);

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
                            let upstream_future = Box::pin(this.upstream.call(request));
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
                            let upstream_future = Box::pin(this.upstream.call(request));
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
                        None => State::CheckConcurrency {
                            request: request.take(),
                        },
                    }
                }
                StateProj::CheckConcurrency { request } => {
                    let request = request.take().expect(POLL_AFTER_READY_ERROR);
                    match this.policy.as_ref() {
                        PolicyConfig::Enabled(config) if config.concurrency.is_some() => {
                            State::ConcurrentPollUpstream {
                                request: Some(request),
                            }
                        }
                        _ => {
                            let upstream_future = Box::pin(this.upstream.call(request));
                            State::PollUpstream { upstream_future }
                        }
                    }
                }
                StateProj::ConcurrentPollUpstream { request } => {
                    let request = request.take().expect(POLL_AFTER_READY_ERROR);
                    let cache_key = this
                        .cache_key
                        .as_ref()
                        .expect("CacheKey not found for concurrency check");
                    match this.concurrency_manager.check(cache_key) {
                        ConcurrencyDecision::Proceed => {
                            let upstream_future = Box::pin(this.upstream.call(request));
                            State::PollUpstream { upstream_future }
                        }
                        ConcurrencyDecision::Await(await_future) => State::AwaitResponse {
                            await_response_future: await_future,
                        },
                    }
                }
                StateProj::AwaitResponse {
                    await_response_future,
                } => {
                    let response = ready!(await_response_future.poll(cx));
                    State::Response {
                        response: Some(response),
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
                        CacheState::Expired(_response) => {
                            *this.cache_status = CacheStatus::Miss;
                            State::CheckConcurrency {
                                request: request.take(),
                            }
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
                        Some(cache_key) => {
                            // Notify waiting requests that response is ready
                            let upstream_result = this
                                .concurrency_manager
                                .complete(cache_key, upstream_result);

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
                    let upstream_response = response.take().expect(POLL_AFTER_READY_ERROR);
                    let cache_context = if *this.cache_enabled {
                        let mut ctx = crate::CacheContext {
                            status: *this.cache_status,
                            ..Default::default()
                        };
                        if let Some(key) = this.cache_key.as_ref() {
                            ctx.key = Some(key.clone());
                        }
                        ctx
                    } else {
                        crate::CacheContext::default()
                    };
                    return Poll::Ready((upstream_response, cache_context));
                }
            };
            debug!("{:?}", &state);
            this.state.set(state);
        }
    }
}
