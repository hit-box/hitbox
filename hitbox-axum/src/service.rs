use std::task::{Context, Poll};

use axum::http::{Request, Response};
use futures::future::BoxFuture;
use tower_service::Service;

use crate::Wrapper;

#[derive(Clone)]
pub struct CacheService<S> {
    service: S,
}

impl<S> CacheService<S> {
    pub fn new(service: S) -> CacheService<S> {
        CacheService { service }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CacheService<S>
    where
        S: Service<Request<ReqBody>, Response=Response<ResBody>> + Clone + Send + 'static,
        S::Future: Send + 'static,
        ReqBody: Send + 'static,
        ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        // self.service.call(wrapper.into_inner())
        let clone = self.service.clone();
        let mut service = std::mem::replace(&mut self.service, clone);

        Box::pin(async move {
            let wrapper = Wrapper { request };
            let res: Response<ResBody> = service.call(wrapper.into_inner()).await?;
            Ok(res)
        })
    }
}
