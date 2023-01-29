use std::{fmt::Debug, sync::Arc};

use futures::Future;
use hitbox::Cacheable;
use hitbox_http::CacheableRequest;
use http::Request;
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

impl<Req, S, B, PollUpstream> Service<Request<Req>> for CacheService<S, B>
where
    S: Service<Request<Req>, Future = PollUpstream>,
    PollUpstream: Future<Output = Result<S::Response, S::Error>>,
    PollUpstream::Output: Debug,

    Request<Req>: Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = FutureResponse<PollUpstream, B>;

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
        dbg!(cache_key);
        FutureResponse::new(self.upstream.call(req), self.backend.clone())
    }
}
