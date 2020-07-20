use actix::prelude::*;
use actix_cache_redis::actor::RedisActor;
use log::{debug, info};

use crate::CacheError;

pub struct Cache {
    enabled: bool,
    pub backend: Addr<RedisActor>,
}

impl Cache {
    pub async fn new() -> Result<Self, CacheError> {
        CacheBuilder::default().build().await
    }
}

impl Actor for Cache {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
        debug!("Cache enabled: {}", self.enabled);
    }
}

pub struct CacheBuilder {
    enabled: bool,
}

impl Default for CacheBuilder {
    fn default() -> CacheBuilder {
        CacheBuilder { enabled: false }
    }
}

impl CacheBuilder {
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub async fn build(&self) -> Result<Cache, CacheError> {
        Ok(Cache {
            enabled: self.enabled,
            backend: actix_cache_redis::actor::RedisActor::new()
                .await
                .map_err(|err| CacheError::BackendError(err.into()))?
                .start(),
        })
    }
}
