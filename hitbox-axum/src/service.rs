use std::task::{Context, Poll};

use axum::http::{Request, Response};
use futures::future::BoxFuture;
use tower_service::Service;

use crate::config::CacheConfig;
use crate::CacheableRequest;
use hitbox::cache::Cacheable;

#[derive(Clone)]
pub struct CacheService<S> {
    service: S,
    cache_config: CacheConfig,
}

impl<S> CacheService<S> {
    pub fn new(service: S, cache_config: CacheConfig) -> CacheService<S> {
        CacheService {
            service,
            cache_config,
        }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CacheService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
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
        let clone = self.service.clone();
        let mut service = std::mem::replace(&mut self.service, clone);
        let cache_config = self.cache_config.clone();

        Box::pin(async move {
            let wrapper = CacheableRequest {
                request,
                cache_config,
            };
            let cache_key = wrapper.cache_key().unwrap_or_default();
            println!("Cache key: {}", cache_key);
            let res: Response<ResBody> = service.call(wrapper.into_inner()).await?;
            Ok(res)
        })
    }
}
