use std::task::Poll;

use futures::{future::BoxFuture, ready, Future};
use hitbox::{cache::CacheableRequest, fsm::CacheFuture};
use hitbox_backend::CachePolicy;
use hitbox_http::CacheableHttpResponse;
use http::Request;
use hyper::{body::to_bytes, Body};
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

    #[pin]
    request_policy_future: Option<BoxFuture<'static, (bool, Request<Body>)>>,
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
            request_policy_future: None,
        }
    }

    pub async fn cache_request_policy(&self, req: Request<Body>) -> CachePolicy<Request<Body>> {
        let uri = req.uri().clone();
        let body = req.into_body();
        let payload = to_bytes(body).await.unwrap();
        let request = Request::builder().uri(uri).body(Body::from(payload));
        CachePolicy::Cacheable(request.unwrap())
    }
}

pub fn wrap_upstream_call<S, U>(
    cacheable_request: CacheableHttpRequest,
    service: &mut S,
) -> impl Future<Output = U>
where
    S: Service,
    S::Future: Future<Output = U>,
{
    service.call(cacheable_request.into_request())
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
        let req = this.req.take().unwrap();

        let cacheable_request = hitbox_http::CacheableHttpRequest::from_request(req);
        let upstream_future = wrap_upstream_call(cacheable_request);
        let cache_future = CacheFuture::new(upstream_future, backend, cacheable_request);
        // let (policy, req) = cacheable_request.cache_policy(&[]).await;

        let f = Box::pin(async move {
            let uri = req.uri().clone();
            let body = req.into_body();
            let payload = to_bytes(body).await.unwrap();
            let request = Request::builder()
                .uri(uri)
                .body(Body::from(payload))
                .unwrap();
            (true, request)
        });
        this.request_policy_future.set(Some(f));
        loop {
            let (should_cache, req) = ready!(this
                .request_policy_future
                .as_pin_mut()
                .take()
                .unwrap()
                .poll(cx));
            dbg!(should_cache);
            this.upstream.set(Some(this.service.call(req)));
            return Poll::Ready(ready!(this.upstream.as_pin_mut().take().unwrap().poll(cx)));
        }
    }
}
