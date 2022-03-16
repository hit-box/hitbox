use std::{future::Future, marker::PhantomData};

use async_trait::async_trait;

use hitbox::{
    runtime::{AdapterResult, RuntimeAdapter},
    CacheError, CacheState, CacheableResponse, settings::Status
};
use hitbox_backend::CacheBackend;

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
impl<In, Out, U, ResFuture, B, 'b> crate::runtime::RuntimeAdapter for FutureAdapter<'b, In, Out, U, B>
where
    Out: CacheableResponse + Send + Sync,
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
        Err(CacheError::DeserializeError)
    }

    async fn poll_cache(
        &self,
    ) -> crate::runtime::AdapterResult<CacheState<Self::UpstreamResult>> {
        Err(CacheError::DeserializeError)
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
