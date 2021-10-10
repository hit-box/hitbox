use hitbox::runtime::{RuntimeAdapter, AdapterResult, EvictionPolicy};
use hitbox::{CacheState, CachedValue};
use tower_service::Service;
use std::marker::PhantomData;
use hitbox::response::CacheableResponse;

pub struct AxumRuntimeAdapter<S, Request>
where
    S: Service<Request>,
{
    pub service: S,
    pub request: PhantomData<Request>,
}

impl<S, Request> AxumRuntimeAdapter<S, Request>
where
    S: Service<Request>
{
    pub fn new(service: S) -> Self {
        Self { service, request: Default::default() }
    }
}

impl<S, Request> RuntimeAdapter for AxumRuntimeAdapter<S, Request>
where
    S: Service<Request>,
    S::Response: CacheableResponse,
{
    type UpstreamResult = <S as Service<Request>>::Response;

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

