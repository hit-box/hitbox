use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{future::BoxFuture, Future, FutureExt};
use hitbox::fsm::Transform;
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse, FromBytes};
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

impl<S, ReqBody, ResBody> Transform<CacheableHttpRequest<ReqBody>, CacheableHttpResponse<ResBody>>
    for Transformer<S, ReqBody>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: FromBytes,
{
    type Future = UpstreamFuture<ResBody>;
    type Response = Result<Response<ResBody>, S::Error>;

    fn upstream_transform(&self, req: CacheableHttpRequest<ReqBody>) -> Self::Future {
        UpstreamFuture::new(self.inner.clone(), req)
    }

    fn response_transform(&self, res: CacheableHttpResponse<ResBody>) -> Self::Response {
        Ok(res.into_response())
    }
}

#[pin_project]
pub struct UpstreamFuture<ResBody> {
    inner_future: BoxFuture<'static, CacheableHttpResponse<ResBody>>,
}

impl<ResBody> UpstreamFuture<ResBody> {
    pub fn new<S, ReqBody>(mut inner_service: S, req: CacheableHttpRequest<ReqBody>) -> Self
    where
        S: Service<Request<ReqBody>, Response = Response<ResBody>> + Send + 'static,
        S::Future: Send,
        ReqBody: Send + 'static,
        ResBody: FromBytes,
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

impl<ResBody> Future for UpstreamFuture<ResBody> {
    type Output = CacheableHttpResponse<ResBody>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner_future.as_mut().poll(cx)
    }
}
