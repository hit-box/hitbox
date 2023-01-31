use std::{
    fmt::{Debug, Write},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{future::BoxFuture, ready, Future};
use hitbox::{
    dev::{BackendError, CacheBackend},
    CacheableResponse, CachedValue,
};
use hitbox_backend::backend::Backend;
use http::{Request, Response};
use pin_project_lite::pin_project;
use serde::Deserialize;
use tower::Service;

pub type CacheResult<R, E> =
    Result<Option<CachedValue<hitbox_http::CacheableResponse<R, E>>>, BackendError>;
pub type PollCache<R, E> = BoxFuture<'static, CacheResult<R, E>>;
pub type PollCache2<R, E> = Pin<Box<dyn Future<Output = CacheResult<R, E>> + Send>>;

pin_project! {
    #[project = StateProj]
    pub enum State<PU, Res, E, Req, PC> 
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
        Reponse {
            #[pin]
            response: Result<Res, E>,
        }
    }
}

impl<PU, Res, E, Req, PC> Debug for State<PU, Res, E, Req, PC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Initial { req: _ } => f.write_str("State::Initial"),
            State::PollCache { poll_cache: _ } => f.write_str("State::PollCache"),
            State::CachePolled => f.write_str("State::CachePolled"),
            State::PollUpstream { poll_upstream: _ } => f.write_str("State::PollUpstream"),
            State::UpstreamPolled => f.write_str("State::UpstreamPolled"),
            State::Reponse { response: _ } => f.write_str("State::Response"),
        }
    }
}

pin_project! {
    pub struct FutureResponse<S, R, B>
    where
        // PollUpstream: Future<Output = Result<Response<R>, E>>,
        B: Backend,
        S: Service<Request<R>>,
        S::Error: 'static,
        S::Response: 'static,
    {
        #[pin]
        state: State<S::Future, S::Response, S::Error, R, PollCache2<S::Response, S::Error>>,
        service: S,
        backend: B,
    }
}

impl<S, R, B> FutureResponse<S, R, B>
where
    // PollUpstream: Future<Output = Result<Response<R>, E>>,
    // PollUpstream: Future + Send + 'static,
    B: Backend + Send + Sync + 'static,
    S: Service<Request<R>>,
{
    pub fn new(service: S, backend: B, req: Request<R>) -> Self {
        FutureResponse {
            state: State::Initial { req: Some(req) },
            service,
            backend,
        }
    }
}

impl<S, R, B> Future for FutureResponse<S, R, B>
where
    // PollUpstream: Future<Output = Result<Response<R>, E>> + Send + 'static,
    B: Backend + Send + Sync + 'static,
    S: Service<Request<R>>,
    S::Error: Debug,
{
    type Output = Result<S::Response, S::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            dbg!(&this.state.as_ref());
            match this.state.as_mut().project() {
                StateProj::Initial { req: _ } => {
                    // let poll_upstream = this
                        // .service
                        // .call(req.take().expect("future polled after resolving"));
                    // let next = State::PollUpstream { poll_upstream };
                    let poll_cache = this
                        .backend
                        .get("test".to_owned());
                    let next = State::PollCache { poll_cache };
                    this.state.set(next);
                }
                StateProj::PollCache { poll_cache } => {
                    let cached = ready!(Pin::new(poll_cache).poll(cx));
                }
                StateProj::PollUpstream { poll_upstream } => {
                    return Poll::Ready(ready!(poll_upstream.poll(cx)))
                }
                _ => unimplemented!(),
            };
            // this.state.set(new_state);
        }
    }
}
