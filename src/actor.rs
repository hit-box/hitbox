use actix::prelude::*;
use actix_cache_backend::Backend;
use actix_cache_redis::actor::RedisActor;
use log::{debug, info};
use std::marker::PhantomData;

use crate::CacheError;

pub struct Cache<B>
where
    B: Backend,
{
    pub(crate) enabled: bool,
    pub(crate) backend: Addr<B>,
}

impl<B> Cache<B>
where
    B: Backend,
{
    pub async fn new() -> Result<Cache<RedisActor>, CacheError> {
        let backend = RedisActor::new()
            .await
            .map_err(|err| CacheError::BackendError(err.into()))?
            .start();
        Ok(CacheBuilder::default().build(backend))
    }

    pub fn builder() -> CacheBuilder<B> {
        CacheBuilder::default()
    }
}

impl<B> Actor for Cache<B>
where
    B: Backend,
{
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
        debug!("Cache enabled: {}", self.enabled);
    }
}

pub struct CacheBuilder<B>
where
    B: Backend + Actor,
{
    enabled: bool,
    _p: PhantomData<B>,
}

impl<B> Default for CacheBuilder<B>
where
    B: Backend,
{
    fn default() -> Self {
        CacheBuilder {
            enabled: true,
            _p: PhantomData::default(),
        }
    }
}

impl<B> CacheBuilder<B>
where
    B: Backend,
{
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn build(self, backend: Addr<B>) -> Cache<B> {
        Cache {
            enabled: self.enabled,
            backend,
        }
    }
}
