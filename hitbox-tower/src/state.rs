use std::{
    fmt::Debug,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{ready, Future};
use pin_project_lite::pin_project;

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
    pub struct FutureResponse<PollUpstream, B>
    where
        PollUpstream: Future,
    {
        #[pin]
        state: State<PollUpstream::Output, PollUpstream>,
        backend: Arc<B>,
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

impl<PollUpstream, B> FutureResponse<PollUpstream, B>
where
    PollUpstream: Future,
{
    pub fn new(poll_upstream: PollUpstream, backend: Arc<B>) -> Self {
        FutureResponse {
            state: State::Inital { poll_upstream },
            backend,
        }
    }
}

impl<PollUpstream, B> Future for FutureResponse<PollUpstream, B>
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
