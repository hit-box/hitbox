use std::{fmt::Debug, sync::Arc, pin::Pin};

use chrono::{Utc, Duration};
use futures::{Future, future::BoxFuture};
use hitbox::{Cacheable, dev::CacheBackend, CachedValue};
use hitbox_http::{CacheableRequest, CacheableResponse};
use http::{Request, Response};
use tower::Service;

use crate::state::FutureResponse;

pub struct CacheService<S, B> {
    upstream: S,
    backend: Arc<B>,
}

impl<S, B> CacheService<S, B> {
    pub fn new(upstream: S, backend: Arc<B>) -> Self {
        CacheService { upstream, backend }
    }
}

impl<S, B> Clone for CacheService<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            upstream: self.upstream.clone(),
            backend: Arc::clone(&self.backend),
        }
    }
}


async fn transitions<F, B, Res, E>(
    poll_upstream: F,
    backend: Arc<B>,
    cache_key: String,
) -> F::Output
where
    F: Future<Output = Result<Response<Res>, E>> + Send + 'static,
    B: CacheBackend + Send + Sync + 'static,

    Res: Send + 'static,
    E: Send + 'static + Debug,
    F::Output: From<hitbox_http::CacheableResponse<Res, E>>,
{
    let cached: Option<CachedValue<hitbox_http::CacheableResponse<Res, E>>> = backend.get(cache_key).await.unwrap();
    dbg!(&cached);
    match cached {
        Some(res) => {
            res.data.into_response()
        },
        None => poll_upstream.await,
    }
}

// impl<Req, S, B, PollUpstream, Body> Service<Request<Req>> for CacheService<S, B>
impl<Req, S, B, Body> Service<Request<Req>> for CacheService<S, B>
where
    S: Service<Request<Req>, Response = Response<Body>> + Send + 'static,
    // PollUpstream: Future<Output = Result<S::Response, S::Error>> + Send + 'static,
    // PollUpstream::Output: Debug,
    B: CacheBackend + Send + Sync + 'static,

    S::Future: Send + 'static,
    S::Error: Send + Sync + 'static + Debug,
    S::Response: Send + 'static,
    Body: Send + 'static,
    
    Request<Req>: Debug,
    <S::Future as Future>::Output: From<hitbox_http::CacheableResponse<Body, S::Error>>,
{
    type Response = Response<Body>;
    type Error = S::Error;
    // type Future = FutureResponse<Pin<Box<dyn Future<Output = Result<S::Response, S::Error>>>>>;
    type Future = BoxFuture<'static, Result<S::Response, S::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Req>) -> Self::Future {
        dbg!(&req);
        let cacheable_request = CacheableRequest::from_request(&req);
        let cache_key = cacheable_request.cache_key().unwrap();
        let poll_upstream = self.upstream.call(req);
        let backend = self.backend.clone();
        dbg!(&cache_key);
        Box::pin(async move {
            // transitions(poll_upstream, self.backend.clone(), cache_key).await

            let cached: Option<CachedValue<hitbox_http::CacheableResponse<Body, S::Error>>> = backend.get(cache_key.clone()).await.unwrap();
            dbg!(&cached);
            let upstream_result = match cached {
                Some(res) => {
                    return res.data.into_response()
                },
                None => poll_upstream.await,
            };
            let cr = CacheableResponse::from_response(upstream_result);
            let cv = CachedValue::new(cr, Utc::now() + Duration::seconds(10));
            dbg!(backend.set(cache_key, &cv, None).await);
            cv.data.into_response()
        })
        // FutureResponse::new(Box::pin(transitions(&mut self.upstream, req, self.backend.clone(), cache_key)))
    }
}
