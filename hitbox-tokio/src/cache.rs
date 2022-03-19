use std::future::Future;

use hitbox::{
    settings::{CacheSettings, Status},
    states::initial::Initial,
    CacheableResponse, CacheError, Cacheable,
};
use hitbox_backend::{BackendError, CacheBackend};
use hitbox_redis::RedisBackend;
use serde::de::DeserializeOwned;

use crate::FutureAdapter;

pub enum CacheServiceState {
    Running,
    Stopped,
}

pub struct Cache<B=RedisBackend> {
    #[allow(dead_code)]
    state: CacheServiceState,
    #[allow(dead_code)]
    backend: B,
}

impl<B> Cache<B> 
where
    B: CacheBackend
{
    #[allow(dead_code)]
    fn new() -> Result<Cache<RedisBackend>, BackendError> {
        Ok(<Cache>::builder()?.build())
    }

    #[allow(dead_code)]
    fn builder() -> Result<CacheBuilder<RedisBackend>, BackendError> {
        Ok(CacheBuilder {
            backend: Some(RedisBackend::new()?),
        })
    }

    #[allow(dead_code)]
    async fn start(&self) -> Result<(), CacheError> {
        Ok(self.backend.start().await?)
    }

    #[allow(dead_code)]
    async fn process<F, Req, Res, ResFuture>(&self, upstream: F, request: Req) -> Result<Res, CacheError>
    where
        Req: Cacheable + Send + Sync,
        F: Fn(Req) -> ResFuture + Send + Sync,
        ResFuture: Future<Output = Res> + Send + Sync,
        Res: Send + Sync + CacheableResponse + std::fmt::Debug,
        <Res as CacheableResponse>::Cached: DeserializeOwned,
        B: CacheBackend + Send + Sync,
    {
        let adapter = FutureAdapter::new(upstream, request, &self.backend)?;
        // let settings = self.settings.clone();
        let settings = CacheSettings {
            cache: Status::Enabled,
            lock: Status::Disabled,
            stale: Status::Disabled,
        };
        let initial_state = Initial::new(settings, adapter);
        Ok(initial_state.transitions().await?)
    }
}

pub struct CacheBuilder<B> {
    backend: Option<B>,
}

impl<B> CacheBuilder<B> {
    #[allow(dead_code)]
    fn backend(backend: B) -> CacheBuilder<B> {
        CacheBuilder {
            backend: Some(backend),
        }
    }

    fn build(self) -> Cache<B> {
        Cache {
            state: CacheServiceState::Stopped,
            backend: self.backend.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hitbox::{CacheError, Cacheable};
    use test_log::test;

    struct Message(i32);

    impl Cacheable for Message {
        fn cache_key(&self) -> Result<String, CacheError> {
            Ok("Message".to_owned())
        }
        fn cache_key_prefix(&self) -> String {
            "Message".to_owned()
        }
        fn cache_ttl(&self) -> u32 {
            20
        }
    }

    async fn upstream_fn(message: Message) -> i32 {
        message.0
    }

    #[test(tokio::test)]
    async fn test_cache_process() {
        let cache = <Cache>::new().unwrap();
        cache.start().await.unwrap();
        let response = cache.process(upstream_fn, Message(42)).await.unwrap();
        dbg!(response);
    }
}
