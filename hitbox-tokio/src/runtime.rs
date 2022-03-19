use std::{future::Future, marker::PhantomData};

use async_trait::async_trait;

use hitbox::{
    runtime::{AdapterResult, RuntimeAdapter},
    CacheError, CacheState, CacheableResponse, Cacheable,
};
use hitbox_backend::CacheBackend;
use serde::de::DeserializeOwned;

pub struct FutureAdapter<'b, In, Out, U, B> 
where
    In: Cacheable,
{
    _response: PhantomData<Out>,
    backend: &'b B,
    request: Option<In>,
    cache_key: String,
    cache_ttl: u32,
    #[allow(dead_code)]
    cache_stale_ttl: u32,
    upstream: U,
}

impl<'b, In, Out, U, B> FutureAdapter<'b, In, Out, U, B> 
where
    In: Cacheable,
{
    pub fn new(upstream: U, request: In, backend: &'b B) -> Result<Self, CacheError> {
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
impl<In, Out, U, ResFuture, B, 'b> crate::runtime::RuntimeAdapter
    for FutureAdapter<'b, In, Out, U, B>
where
    Out: CacheableResponse + Send + Sync,
    <Out as CacheableResponse>::Cached: DeserializeOwned,
    U: Send + Sync + Fn(In) -> ResFuture,
    ResFuture: Future<Output = Out> + Send,
    In: Cacheable + Send + Sync,
    B: CacheBackend + Send + Sync,
{
    type UpstreamResult = Out;
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
        println!("huy");
        let request = self.request.take();
        let request = request.ok_or_else(|| {
            CacheError::CacheKeyGenerationError("Request already sent to upstream".to_owned())
        })?;
        Ok((self.upstream)(request).await)
    }

    fn eviction_settings(&self) -> hitbox_backend::EvictionPolicy {
        hitbox_backend::EvictionPolicy::Ttl(hitbox_backend::TtlSettings {
            ttl: 42,
            stale_ttl: 24,
        })
    }
}
