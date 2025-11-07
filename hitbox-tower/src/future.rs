use std::{
    fmt::Debug,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, future::BoxFuture};
use hitbox::fsm::Transform;
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse, FromBytes};
use http::{Request, Response};
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
            _req: PhantomData,
        }
    }
}

impl<S, ReqBody, ResBody>
    Transform<CacheableHttpRequest<ReqBody>, Result<CacheableHttpResponse<ResBody>, S::Error>>
    for Transformer<S, ReqBody>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: FromBytes,
    // debug bounds
    S::Error: Debug,
{
    type Future = UpstreamFuture<ResBody, S::Error>;
    type Response = Result<Response<ResBody>, S::Error>;

    fn upstream_transform(&self, req: CacheableHttpRequest<ReqBody>) -> Self::Future {
        UpstreamFuture::new(self.inner.clone(), req)
    }

    fn response_transform(
        &self,
        res: Result<CacheableHttpResponse<ResBody>, S::Error>,
        cache_status: Option<hitbox::CacheStatus>,
    ) -> Self::Response {
        res.map(|cacheable_response| {
            let mut response = cacheable_response.into_response();
            if let Some(status) = cache_status {
                let status_value = match status {
                    hitbox::CacheStatus::Hit => "HIT",
                    hitbox::CacheStatus::Miss => "MISS",
                };
                response
                    .headers_mut()
                    .insert("X-Cache-Status", status_value.parse().unwrap());
            }
            response
        })
    }

    fn error_transform(&self, error: hitbox::PredicateError) -> Self::Response {
        use bytes::Bytes;

        // Convert predicate error to HTTP 500 response
        let error_message = error.to_string();
        let body = ResBody::from_bytes(Bytes::from(error_message));

        let response = Response::builder()
            .status(500)
            .header("Content-Type", "text/plain")
            .body(body)
            .expect("Failed to build error response");

        Ok(response)
    }
}

#[pin_project]
pub struct UpstreamFuture<ResBody, E> {
    inner_future: BoxFuture<'static, Result<CacheableHttpResponse<ResBody>, E>>,
}

impl<ResBody, E> UpstreamFuture<ResBody, E> {
    pub fn new<S, ReqBody>(mut inner_service: S, req: CacheableHttpRequest<ReqBody>) -> Self
    where
        S: Service<Request<ReqBody>, Response = Response<ResBody>, Error = E> + Send + 'static,
        S::Future: Send,
        ReqBody: Send + 'static,
        ResBody: FromBytes,
        // debug bounds
        S::Error: Debug,
    {
        let inner_future = Box::pin(async move {
            let res = inner_service.call(req.into_request()).await;
            // CacheableHttpResponse::from_response(res.unwrap())
            // match &res {
            //     Ok(res) => {
            //         dbg!(res.status());
            //     }
            //     Err(err) => {
            //         dbg!(err);
            //     }
            // };
            res.map(CacheableHttpResponse::from_response)
        });
        UpstreamFuture { inner_future }
    }
}

impl<ResBody, E> Future for UpstreamFuture<ResBody, E> {
    type Output = Result<CacheableHttpResponse<ResBody>, E>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner_future.as_mut().poll(cx)
    }
}
