use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{future::BoxFuture, Future, FutureExt};
use hitbox::fsm::Transform;
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse};
use http::{Request, Response};
use hyper::Body;
use pin_project::pin_project;
use tower::Service;

pub struct Transformer<S> {
    inner: S,
}

impl<S> Transformer<S> {
    pub fn new(inner: S) -> Self {
        Transformer { inner }
    }
}

impl<S> Transform<CacheableHttpRequest, CacheableHttpResponse> for Transformer<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Future = UpstreamFuture;
    type Response = Result<Response<Body>, S::Error>;

    fn upstream_transform(&self, req: CacheableHttpRequest) -> Self::Future {
        UpstreamFuture::new(self.inner.clone(), req)
    }

    fn response_transform(&self, res: CacheableHttpResponse) -> Self::Response {
        Ok(res.into_response())
    }
}

#[pin_project]
pub struct UpstreamFuture {
    inner_future: BoxFuture<'static, CacheableHttpResponse>,
}

impl UpstreamFuture {
    pub fn new<S>(mut inner_service: S, req: CacheableHttpRequest) -> Self
    where
        S: Service<Request<Body>, Response = Response<Body>> + Send + 'static,
        S::Future: Send,
    {
        let inner_future = Box::pin(async move {
            let res = inner_service.call(req.into_request()).await;
            match res {
                Ok(res) => CacheableHttpResponse::from_response(res),
                _ => unimplemented!(),
            }
        });
        UpstreamFuture { inner_future }
    }
}

impl Future for UpstreamFuture {
    type Output = CacheableHttpResponse;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner_future.as_mut().poll(cx)
    }
}
