use actix::prelude::*;
use actix_cache_redis::actor::RedisActor;
use actix_cache_backend::Backend;
use log::{debug, info};

use crate::CacheError;

use actix_cache_backend::{Get, Set};

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
    B::Actor: Handler<Get> + Handler<Set>,
{
    pub async fn new(backend: Addr<B>) -> Result<Self, CacheError> {
        // CacheBuilder::<B>::default().build().await
        // let backend = RedisActor::new()
            // .await
            // .map_err(|err| CacheError::BackendError(err.into()))?
            // .start();
        Ok(Cache { enabled: true, backend })
    }

    // pub fn builder() -> CacheBuilder<RedisActor> {
        // CacheBuilder::default()
    // }
}

impl<B> Actor for Cache<B> 
where
    B: Actor + Backend,
{
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
        debug!("Cache enabled: {}", self.enabled);
    }
}

// pub struct CacheBuilder<B> {
    // enabled: bool,
    // backend: Option<B>,
// }

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
