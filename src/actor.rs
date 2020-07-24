use actix::prelude::*;
use actix_cache_redis::actor::RedisActor;
use actix_cache_backend::Backend;
use log::{debug, info};

use crate::CacheError;

pub struct Cache<B> 
where 
    B: Backend,
{
    pub (crate) enabled: bool,
    pub (crate) backend: Addr<B>,
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

use std::marker::PhantomData;

pub struct CacheBuilder<B> 
where
    B: Backend + Actor
{
    enabled: bool,
    _p: PhantomData<B>,
}

impl<B> Default for CacheBuilder<B>
where
    B: Backend
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

// impl<B> Default for CacheBuilder<B> {
    // fn default() -> CacheBuilder<B> {
        // CacheBuilder { enabled: true, backend: None}
    // }
// }

// use actix::dev::ToEnvelope;

// impl<B> CacheBuilder<B>
// where
    // B: Actor + Backend,
    // B::Actor: Handler<Get> + Handler<Set>,
    // // <B as Actor>::Context: ToEnvelope<B, Get> + ToEnvelope<B, Set>,
// {
    // pub fn enabled(mut self, enabled: bool) -> CacheBuilder<B> {
        // self.enabled = enabled;
        // self
    // }

    // pub async fn build(&self) -> Result<Cache<B>, CacheError> 
    // {
        // Ok(Cache {
            // enabled: self.enabled,
            // backend: RedisActor::new()
                // .await
                // .map_err(|err| CacheError::BackendError(err.into()))?
                // .start(),
        // })
    // }
// }
