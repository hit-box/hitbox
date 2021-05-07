use crate::runtime::{RuntimeAdapter, AdapterResult};
use crate::{Cacheable, QueryCache, CachedValue, CacheState};
use crate::response::{CacheableResponse, CachePolicy};
use actix::dev::{MessageResponse, ToEnvelope};
use actix::{Actor, Addr, Handler, Message};
use hitbox_backend::{Backend, Get};
use serde::de::DeserializeOwned;

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
    <B as Actor>::Context: ToEnvelope<B, Get>,
    T: CacheableResponse<Cached = U>,
    U: DeserializeOwned,
{
    type UpstreamResult = T;

    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult> {
        let message = self.message.message.clone();
        let upstream = self.message.upstream.clone();
        Box::pin(async move { Ok(upstream.send(message).await.unwrap()) })
    }

    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>> {
        let backend = self.backend.clone();
        let cache_key = self.message.cache_key().unwrap();
        Box::pin(async move {
            let cached_value = backend.send(Get { key: cache_key }).await.unwrap().unwrap();
            CacheState::from_bytes(cached_value.as_ref())
        })
    }
    // fn update_cache(&self, data: T) -> AdapterResult<T> {
    //     let r = match data.into_policy() {
    //         CachePolicy::Cacheable(value) => Ok(value),
    //         CachePolicy::NonCacheable(value) => Err(value)
    //     };
    // }
}
