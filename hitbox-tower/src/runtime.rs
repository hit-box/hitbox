use std::{future::Future, marker::PhantomData};

use async_trait::async_trait;

use hitbox::{
    runtime::{AdapterResult, RuntimeAdapter},
    CacheError, CacheState, CacheableResponse, Cacheable,
};
use hitbox_backend::CacheBackend;
use serde::de::DeserializeOwned;

pub struct TowerAdapter<'b, Request, Response, Service, B> 
where
    Request: Cacheable,
{
    _response: PhantomData<Response>,
    backend: &'b B,
    upstream: Service,
    request: Option<Request>,
    cache_key: String,
    cache_ttl: u32,
    cache_stale_ttl: u32,
}

impl<'b, Request, Response, Service, B> FutureAdapter<'b, Request, Response, Service, B>
where
    Request: Cacheable,
{
    pub fn new(upstream: U, request: Request, backend: &'b B) -> Result<Self, CacheError> {
        Ok(Self {
            cache_key: request.cache_key()?,
            cache_ttl: request.cache_ttl(),
            cache_stale_ttl: request.cache_stale_ttl(),
            request: Some(request),
            upstream,
            backend,
            _response: PhantomData::default(),
        })
    }
}

#[async_trait]
impl<Request, Response, Service, B, 'b> crate::runtime::RuntimeAdapter for FutureAdapter<'b, Request, Response, U, B>
where
    Request: Cacheable + Send + Sync,
{
    type UpstreamResult = Response;
    async fn update_cache<'a>(
        &self,
        cached_value: &'a hitbox_backend::CachedValue<Self::UpstreamResult>,
    ) -> crate::runtime::AdapterResult<()> {
        Ok(self
            .backend
            .set(self.cache_key.clone(), cached_value, Some(self.cache_ttl))
            .await?)
    }

    async fn poll_cache(&self) -> crate::runtime::AdapterResult<CacheState<Self::UpstreamResult>> {
        Ok(self.backend.get(self.cache_key.clone()).await?.into())
    }

    async fn poll_upstream(&mut self) -> crate::runtime::AdapterResult<Self::UpstreamResult> {
        let request = self.request.take();
        let request = request.ok_or_else(|| {
            CacheError::CacheKeyGenerationError("Request already sent to upstream".to_owned())
        })?;
        Ok(self.upstream.call(request).await)
    }

    fn eviction_settings(&self) -> hitbox_backend::EvictionPolicy {
        let ttl_settings = hitbox_backend::TtlSettings {
            ttl: self.cache_ttl,
            stale_ttl: self.cache_stale_ttl,
        };
        hitbox_backend::EvictionPolicy::Ttl(ttl_settings)
    }
}
