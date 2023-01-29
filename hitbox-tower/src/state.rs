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
    #[project = StateProj]
    pub enum State<Res, PollUpstream> {
        Inital {
            #[pin]
            poll_upstream: PollUpstream
        },
        UpstreamPolled {
            upstream_result: Option<Res>,
        },
    }
}

pin_project! {
    pub struct FutureResponse<PollUpstream>
    where
        PollUpstream: Future,
    {
        #[pin]
        state: State<PollUpstream::Output, PollUpstream>,
    }
}

impl<Res, PollUpstream> Debug for State<Res, PollUpstream> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Inital { poll_upstream: _ } => f.write_str("State::Initial"),
            State::UpstreamPolled { upstream_result: _ } => f.write_str("State::UpstreamPolled"),
        }
    }
}

impl<PollUpstream> FutureResponse<PollUpstream>
where
    PollUpstream: Future,
{
    fn new(poll_upstream: PollUpstream) -> Self {
        FutureResponse {
            state: State::Inital { poll_upstream },
        }
    }
}

impl<PollUpstream> Future for FutureResponse<PollUpstream>
where
    PollUpstream: Future,
    PollUpstream::Output: Debug,
{
    type Output = PollUpstream::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            dbg!(&this.state.as_ref());
            match this.state.as_mut().project() {
                StateProj::Inital { poll_upstream } => {
                    let upstream_result = ready!(poll_upstream.poll(cx));
                    this.state.set(State::UpstreamPolled {
                        upstream_result: Some(upstream_result),
                    });
                }
                StateProj::UpstreamPolled { upstream_result } => {
                    return Poll::Ready(upstream_result.take().unwrap());
                }
            }
        }
    }
}

impl<Req, S, B, PollUpstream> Service<Request<Req>> for CacheService<S, B>
where
    S: Service<Request<Req>, Future = PollUpstream>,
    PollUpstream: Future<Output = Result<S::Response, S::Error>>,
    PollUpstream::Output: Debug,

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
        FutureResponse::new(self.upstream.call(req))
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
