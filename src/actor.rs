use actix::prelude::*;
use log::{info, debug};

pub struct Cache {
    enabled: bool,
}

impl Default for Cache {
    fn default() -> Self {
        CacheBuilder::default().build()
    }
}

impl Cache {
    pub fn new() -> Self {
        Cache::default()
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
        CacheBuilder {
            enabled: false,
        }
    }
}

impl CacheBuilder {
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn build(&self) -> Cache {
        Cache {
            enabled: self.enabled,
        }
    }
}
