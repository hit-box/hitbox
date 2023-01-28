use std::{borrow::Cow, future::Future, marker::PhantomData};

use async_trait::async_trait;

use hitbox::{
    runtime::{AdapterResult, RuntimeAdapter},
    CacheError, CacheState, Cacheable, CacheableResponse,
};
use hitbox_backend::CacheBackend;
use serde::de::DeserializeOwned;

pub struct FutureAdapter<'b, In, Out, U, B>
where
    In: Cacheable,
{
    _response: PhantomData<Out>,
    backend: &'b B,
    upstream: U,
    request: Option<In>,
    cache_key: String,
    cache_ttl: u32,
    cache_stale_ttl: u32,
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
    U: Send + Sync + FnMut(In) -> ResFuture,
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
        let request = self.request.take();
        let request = request.ok_or_else(|| {
            CacheError::CacheKeyGenerationError("Request already sent to upstream".to_owned())
        })?;
        Ok((self.upstream)(request).await)
    }

    fn eviction_settings(&self) -> hitbox_backend::EvictionPolicy {
        let ttl_settings = hitbox_backend::TtlSettings {
            ttl: self.cache_ttl,
            stale_ttl: self.cache_stale_ttl,
        };
        hitbox_backend::EvictionPolicy::Ttl(ttl_settings)
    }

    fn upstream_name(&self) -> Cow<'static, str> {
        std::any::type_name::<U>().into()
    }

    fn message_name(&self) -> Cow<'static, str> {
        self.cache_key.clone().into()
    }
}
