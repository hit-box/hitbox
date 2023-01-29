use std::{
    fmt::Debug,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{ready, Future};
use hitbox::dev::CacheBackend;
use hitbox_redis::RedisBackend;
use hitbox_tokio::FutureAdapter;
use http::Request;
use pin_project_lite::pin_project;
use tower::{Layer, Service};

pub struct CacheService<S, B> {
    upstream: S,
    backend: Arc<B>,
}

impl<S, B> Clone for CacheService<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            upstream: self.upstream.clone(),
            backend: Arc::clone(&self.backend),
        }
    }
}

pin_project! {
    pub struct FutureResponse<PollUpstream> {
        #[pin]
        state: CacheState<PollUpstream>,
    }
}

pin_project! {
    #[project = CacheStateProj]
    enum CacheState<PollUpstream> {
        // CachePoll {
            // poll_cache: PollCache
        // },
        UpstreamPoll {
            #[pin]
            poll_upstream: PollUpstream
        }
    }
}

impl<PollUpstream> Future for FutureResponse<PollUpstream>
where
    PollUpstream: Future,
{
    type Output = PollUpstream::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        
        // let adapter = FutureAdapter::new(poll_upstream, req, self.backend);

        match this.state.as_mut().project() {
            CacheStateProj::UpstreamPoll { poll_upstream } => {
                return Poll::Ready(ready!(poll_upstream.poll(cx)))
            }
        }
    }
}

impl<Req, S, B, PollUpstream> Service<Request<Req>> for CacheService<S, B>
where
    S: Service<Request<Req>, Future = PollUpstream>,
    PollUpstream: Future<Output = Result<S::Response, S::Error>>,

    Request<Req>: Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = FutureResponse<PollUpstream>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Req>) -> Self::Future {
        dbg!(&req);
        let state = CacheState::UpstreamPoll {
            poll_upstream: self.upstream.call(req),
        };
        FutureResponse { state }
    }
}

pub struct Cache<B> {
    backend: Arc<B>,
}

impl<B> Clone for Cache<B> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
        }
    }
}

impl<S, B> Layer<S> for Cache<B> {
    type Service = CacheService<S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService {
            upstream,
            backend: self.backend.clone(),
        }
    }
}

impl<B> Cache<B>
where
    B: CacheBackend,
{
    pub fn new() -> Cache<RedisBackend> {
        Cache {
            backend: Arc::new(RedisBackend::new().unwrap()),
        }
    }
}
