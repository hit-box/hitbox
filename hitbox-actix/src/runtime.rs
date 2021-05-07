use hitbox::runtime::{RuntimeAdapter, AdapterResult};
use hitbox::{Cacheable, CacheState, CachedValue};
use hitbox::response::CacheableResponse;
use actix::dev::{MessageResponse, ToEnvelope};
use actix::{Actor, Addr, Handler, Message};
use hitbox_backend::{Backend, Get, Set};
use serde::de::DeserializeOwned;
use crate::QueryCache;
use serde::Serialize;

pub struct ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
    B: Backend,
{
    message: QueryCache<A, M>,
    backend: Addr<B>,
}

impl<A, M, B> ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
    B: Backend,
{
    pub fn new(message: QueryCache<A, M>, backend: Addr<B>) -> Self {
        Self { message, backend }
    }
}

impl<A, M, T, B, U> RuntimeAdapter for ActixAdapter<A, M, B>
where
    A: Actor + Handler<M>,
    A::Context: ToEnvelope<A, M>,
    M: Message<Result = T> + Cacheable + Send + Clone + 'static,
    M::Result: MessageResponse<A, M> + Send,
    B: Backend,
    <B as Actor>::Context: ToEnvelope<B, Get> + ToEnvelope<B, Set>,
    T: CacheableResponse<Cached = U>,
    U: DeserializeOwned + Serialize,
{
    type UpstreamResult = T;

    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult> {
        let message = self.message.message.clone();
        let upstream = self.message.upstream.clone();
        Box::pin(async move { Ok(upstream.send(message).await?) })
    }

    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>> {
        let backend = self.backend.clone();
        let cache_key = self.message.cache_key();  // @TODO: Please, don't recalculate cache key multiple times.
        Box::pin(async move {
            let key = cache_key?;
            let cached_value = backend.send(Get { key }).await??;
            CacheState::from_bytes(cached_value.as_ref())
        })
    }

    fn update_cache(&self, cached_value: &CachedValue<Self::UpstreamResult>) -> AdapterResult<()> {
        // let serialized = serde_json::to_vec(&cached_value);
        let serialized = cached_value.serialize();
        let backend = self.backend.clone();
        let cache_key = self.message.cache_key();  // @TODO: Please, don't recalculate cache key multiple times.
        dbg!(&serialized);
        Box::pin(async move { 
            let key = cache_key?;
            backend.send(Set { key, value: serialized, ttl: None }).await.unwrap().unwrap();
            Ok(())
        })
    }
}
