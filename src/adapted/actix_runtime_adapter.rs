use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::adapted::AdapterResult;
use crate::{Cacheable, QueryCache};
use actix::dev::{MessageResponse, ToEnvelope};
use actix::{Actor, Addr, Handler, Message};
use actix_cache_backend::{Backend, Get};

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
            let cached_value = backend
                .send(Get { key:  cache_key})
                .await
                .unwrap()
                .unwrap();
            CacheState::from_bytes(cached_value.as_ref())
        })
    }
}

use serde::{de::DeserializeOwned, Deserialize};
use crate::response::CacheableResponse;
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct CachedValue<T> {
    data: T,
    expired: DateTime<Utc>,
}

impl<T> CachedValue<T> {
    fn from_inner<U>(cached_data: CachedValue<U>) -> Self
    where
        T: CacheableResponse<Cached = U>,
    {
        Self {
            data: T::from_cached(cached_data.data),
            expired: cached_data.expired,
        }
    }
    pub fn into_inner(self) -> T {
        self.data
    }
}

pub enum CacheState<T> {
    Actual(CachedValue<T>),
    Stale(CachedValue<T>),
    Miss,
}

impl<T, U> CacheState<T> 
where
    T: CacheableResponse<Cached = U>,
    U: DeserializeOwned,
{
    pub fn from_bytes(bytes: Option<&Vec<u8>>) -> Result<Self, crate::CacheError> {
        let cached_data = bytes
            .map(|bytes| serde_json::from_slice::<CachedValue<U>>(bytes).unwrap());
        Ok(Self::from(cached_data))
    }
}

impl<T, U> From<Option<CachedValue<U>>> for CacheState<T>
where
    T: CacheableResponse<Cached = U>,
{
    fn from(cached_data: Option<CachedValue<U>>) -> Self {
        match cached_data {
            Some(cached_data) => Self::Actual(CachedValue::from_inner(cached_data)),
            None => Self::Miss,
        }
    }
}
