//! [hitbox::runtime::RuntimeAdapter] implementation for Actix runtime.
use actix::dev::{MessageResponse, ToEnvelope};
use actix::{Actor, Addr, Handler, Message};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::warn;
use async_trait::async_trait;

use hitbox::runtime::{AdapterResult, EvictionPolicy, RuntimeAdapter, TtlSettings};
use hitbox::{CacheError, CacheState, Cacheable, CacheableResponse, CachedValue};
use hitbox_backend::{Backend, CacheBackend, Get, Set};

use crate::QueryCache;

use std::sync::Arc;

/// [`RuntimeAdapter`] for Actix runtime.
pub struct ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    M: Message + Cacheable + Send + Sync,
    M::Result: MessageResponse<A, M> + Send,
    B: CacheBackend,
{
    message: Option<QueryCache<A, M>>,
    cache_key: String,
    cache_ttl: u32,
    cache_stale_ttl: u32,
    backend: Arc<B>,
}

impl<A, M, B> ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    M: Message + Cacheable + Send + Sync,
    M::Result: MessageResponse<A, M> + Send,
    B: CacheBackend,
{
    /// Creates new instance of Actix runtime adapter.
    pub fn new(message: QueryCache<A, M>, backend: Arc<B>) -> Result<Self, CacheError> {
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

#[async_trait]
impl<A, M, T, B, U> RuntimeAdapter for ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    A::Context: ToEnvelope<A, M>,
    M: Message<Result = T> + Cacheable + Send + 'static + Sync,
    M::Result: MessageResponse<A, M> + Send,
    B: CacheBackend + 'static + Send + Sync,
    T: CacheableResponse<Cached = U> + 'static + Sync,
    U: DeserializeOwned + Serialize,
    Self: Send + Sync,
{
    type UpstreamResult = T;

    async fn poll_upstream(&mut self) -> AdapterResult<Self::UpstreamResult> {
        let message = self.message.take();
        let message = message.ok_or_else(|| {
            CacheError::CacheKeyGenerationError("Message already sent to upstream".to_owned())
        })?;
        Ok(message.upstream.send(message.message).await?)
    }

    async fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>> {
        let backend = self.backend.clone();
        let cache_key = self.cache_key.clone();
        backend
            .get(cache_key)
            .await
            .map(CacheState::from)
            .map_err(CacheError::from)
    }

    async fn update_cache<'a>(
        &self,
        cached_value: &'a CachedValue<Self::UpstreamResult>,
    ) -> AdapterResult<()> {
        let ttl = self.cache_ttl;
        let backend = self.backend.clone();
        let cache_key = self.cache_key.clone();
        backend
            .set(cache_key, cached_value, Some(ttl))
            .await
            .map_err(|err| {
                warn!("Updating cache error {}", err);
                CacheError::from(err)
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
