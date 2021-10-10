use hitbox::runtime::{RuntimeAdapter, AdapterResult, EvictionPolicy};
use hitbox::{CacheState, CachedValue};
use tower_service::Service;
use std::marker::PhantomData;
use hitbox::response::CacheableResponse;
use crate::cacheable_response::AxumCacheableResponse;
use axum::http::{Request, Response};
use serde::Serialize;

pub struct AxumRuntimeAdapter<S, ReqBody>
where
    S: Service<Request<ReqBody>>,
{
    pub service: S,
    pub request: PhantomData<ReqBody>,
}

impl<S, ReqBody> AxumRuntimeAdapter<S, ReqBody>
where
    S: Service<Request<ReqBody>>,
{
    pub fn new(service: S) -> Self {
        Self { service, request: Default::default() }
    }
}

impl<S, ReqBody, ResBody> RuntimeAdapter for AxumRuntimeAdapter<S, ReqBody>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Send,
    ResBody: Serialize,
{
    type UpstreamResult = AxumCacheableResponse<ResBody>;

    fn poll_upstream(&mut self) -> AdapterResult<Self::UpstreamResult> {
        todo!()
    }

    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>> {
        todo!()
    }

    fn update_cache(&self, cached_value: &CachedValue<Self::UpstreamResult>) -> AdapterResult<()> {
        todo!()
    }

    fn eviction_settings(&self) -> EvictionPolicy {
        todo!()
    }
}

