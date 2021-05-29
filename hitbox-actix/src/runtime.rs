//! [hitbox::runtime::RuntimeAdapter] implementation for Actix runtime.
use actix::dev::{MessageResponse, ToEnvelope};
use actix::{Actor, Addr, Handler, Message};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::warn;

use hitbox::response::CacheableResponse;
use hitbox::runtime::{AdapterResult, EvictionPolicy, RuntimeAdapter, TtlSettings};
use hitbox::{CacheError, CacheState, Cacheable, CachedValue};
use hitbox_backend::{Backend, Get, Set};

use crate::QueryCache;

/// [`RuntimeAdapter`] for Actix runtime.
pub struct ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
    B: Backend,
{
    message: Option<QueryCache<A, M>>,
    cache_key: String,
    cache_ttl: u32,
    cache_stale_ttl: u32,
    backend: Addr<B>,
}

impl<A, M, B> ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
    B: Backend,
{
    /// Creates new instance of Actix runtime adapter.
    pub fn new(message: QueryCache<A, M>, backend: Addr<B>) -> Result<Self, CacheError> {
        let cache_key = message.cache_key()?;
        let cache_stale_ttl = message.message.cache_ttl();
        let cache_ttl = message.message.cache_ttl();
        Ok(Self {
            message: Some(message),
            backend,
            cache_key,
            cache_ttl,
            cache_stale_ttl,
        })
    }
}

impl<A, M, T, B, U> RuntimeAdapter for ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    A::Context: ToEnvelope<A, M>,
    M: Message<Result = T> + Cacheable + Send + 'static,
    M::Result: MessageResponse<A, M> + Send,
    B: Backend,
    <B as Actor>::Context: ToEnvelope<B, Get> + ToEnvelope<B, Set>,
    T: CacheableResponse<Cached = U> + 'static,
    U: DeserializeOwned + Serialize,
{
    type UpstreamResult = T;

    fn poll_upstream(&mut self) -> AdapterResult<Self::UpstreamResult> {
        let message = self.message.take();
        Box::pin(async move {
            let message = message.ok_or_else(|| CacheError::CacheKeyGenerationError(
                "Message already sent to upstream".to_owned(),
            ))?;
            Ok(message.upstream.send(message.message).await?)
        })
    }

    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>> {
        let backend = self.backend.clone();
        let cache_key = self.cache_key.clone();
        Box::pin(async move {
            let cached_value = backend.send(Get { key: cache_key }).await??;
            CacheState::from_bytes(cached_value.as_ref())
        })
    }

    fn update_cache(&self, cached_value: &CachedValue<Self::UpstreamResult>) -> AdapterResult<()> {
        let serialized = cached_value.serialize();
        let ttl = self.cache_ttl;
        let backend = self.backend.clone();
        let cache_key = self.cache_key.clone();
        Box::pin(async move {
            let serialized = serialized?;
            let _ = backend
                .send(Set {
                    key: cache_key,
                    value: serialized,
                    ttl: Some(ttl),
                })
                .await
                .map_err(|error| warn!("Updating Cache Error {}", error))
                .and_then(|value| value.map_err(|error| warn!("Updating Cache Error. {}", error)));
            Ok(())
        })
    }
    fn eviction_settings(&self) -> EvictionPolicy {
        let ttl_settings = TtlSettings {
            ttl: self.cache_ttl,
            stale_ttl: self.cache_stale_ttl,
        };
        EvictionPolicy::Ttl(ttl_settings)
    }
}
