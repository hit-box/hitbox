use std::{
    marker::PhantomData,
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

pub struct Transformer<S, ReqBody> {
    inner: S,
    _req: PhantomData<ReqBody>,
}

impl<S, ReqBody> Transformer<S, ReqBody> {
    pub fn new(inner: S) -> Self {
        Transformer {
            inner,
            _req: PhantomData::default(),
        }
    }
}

impl<S, ReqBody> Transform<CacheableHttpRequest<ReqBody>, CacheableHttpResponse>
    for Transformer<S, ReqBody>
where
    S: Service<Request<ReqBody>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
{
    type Future = UpstreamFuture;
    type Response = Result<Response<Body>, S::Error>;

    fn upstream_transform(&self, req: CacheableHttpRequest<ReqBody>) -> Self::Future {
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
    pub fn new<S, ReqBody>(mut inner_service: S, req: CacheableHttpRequest<ReqBody>) -> Self
    where
        S: Service<Request<ReqBody>, Response = Response<Body>> + Send + 'static,
        S::Future: Send,
        ReqBody: Send + 'static,
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
