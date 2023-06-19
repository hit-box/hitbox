use std::task::Poll;

use futures::{ready, Future};
use hitbox_http::HttpResponse;
use http::Request;
use hyper::Body;
use pin_project::pin_project;
use tower::Service;

#[pin_project]
pub struct CacheFutureAdapter<U, S>
where
    U: Future,
    S: Service<Request<Body>>,
{
    #[pin]
    upstream: Option<U>,
    req: Option<Request<Body>>,
    service: S,
}

impl<U, S> CacheFutureAdapter<U, S>
where
    U: Future,
    S: Service<Request<Body>, Future = U> + Clone,
{
    pub fn new(service: &mut S, req: Request<Body>) -> Self {
        Self {
            service: service.clone(),
            req: Some(req),
            upstream: None,
        }
    }
}

impl<U, S> Future for CacheFutureAdapter<U, S>
where
    U: Future,
    S: Service<Request<Body>, Future = U>,
{
    type Output = U::Output;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut this = self.project();
        this.upstream
            .set(Some(this.service.call(this.req.take().unwrap())));
        loop {
            return Poll::Ready(ready!(this.upstream.as_pin_mut().take().unwrap().poll(cx)));
        }
    }
}
