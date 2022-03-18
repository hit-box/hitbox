use std::future::Future;

use hitbox::{
    settings::{CacheSettings, Status},
    states::initial::Initial,
    CacheableResponse, CacheError,
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
    state: CacheServiceState,
    backend: B,
}

impl<B> Cache<B> 
where
    B: CacheBackend
{
    fn new() -> Result<Cache<RedisBackend>, BackendError> {
        Ok(<Cache>::builder()?.build())
    }

    fn builder() -> Result<CacheBuilder<RedisBackend>, BackendError> {
        Ok(CacheBuilder {
            backend: Some(RedisBackend::new()?),
        })
    }

    async fn start(&self) -> Result<(), CacheError> {
        Ok(self.backend.start().await?)
    }

    async fn process<F, Req, Res, ResFuture>(&self, upstream: F, request: Req) -> Res
    where
        Req: Send + Sync,
        F: Fn(Req) -> ResFuture + Send + Sync,
        ResFuture: Future<Output = Res> + Send + Sync,
        Res: Send + Sync + CacheableResponse + std::fmt::Debug,
        <Res as CacheableResponse>::Cached: DeserializeOwned,
        B: CacheBackend + Send + Sync,
    {
        let adapter_result = FutureAdapter::new(upstream, request, &self.backend);
        // let settings = self.settings.clone();
        let settings = CacheSettings {
            cache: Status::Enabled,
            lock: Status::Disabled,
            stale: Status::Disabled,
        };
        let initial_state = Initial::new(settings, adapter_result);
        initial_state.transitions().await.unwrap()
    }
}

pub struct CacheBuilder<B> {
    backend: Option<B>,
}

impl<B> CacheBuilder<B> {
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
            2
        }
    }

    async fn upstream_fn(message: Message) -> i32 {
        message.0
    }

    #[test(tokio::test)]
    async fn test_cache_process() {
        let cache = <Cache>::new().unwrap();
        cache.start().await.unwrap();
        let response = cache.process(upstream_fn, Message(42)).await;
        dbg!(response);
    }
}
