use std::{future::Future, marker::PhantomData};

use async_trait::async_trait;

use hitbox::{
    runtime::{AdapterResult, RuntimeAdapter},
    settings::Status,
    CacheError, CacheState, CacheableResponse,
};
use hitbox_backend::CacheBackend;
use serde::de::DeserializeOwned;

pub struct FutureAdapter<'b, In, Out, U, B> {
    _response: PhantomData<Out>,
    backend: &'b B,
    request: Option<In>,
    upstream: U,
}

impl<'b, In, Out, U, B> FutureAdapter<'b, In, Out, U, B> {
    pub fn new(upstream: U, request: In, backend: &'b B) -> Self {
        Self {
            request: Some(request),
            upstream,
            backend,
            _response: PhantomData::default(),
        }
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
    In: Send + Sync,
    B: CacheBackend + Send + Sync,
{
    type UpstreamResult = Out;
    async fn update_cache<'a>(
        &self,
        cached_value: &'a hitbox_backend::CachedValue<Self::UpstreamResult>,
    ) -> crate::runtime::AdapterResult<()> {
        Ok(self
            .backend
            .set("test_key".to_owned(), cached_value, Some(42))
            .await?)
    }

    async fn poll_cache(&self) -> crate::runtime::AdapterResult<CacheState<Self::UpstreamResult>> {
        Ok(self.backend.get("test_key".to_owned()).await?.into())
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
