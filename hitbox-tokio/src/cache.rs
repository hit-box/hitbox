use std::future::Future;

use hitbox::{CacheableResponse, states::initial::Initial, settings::{CacheSettings, Status}};
use hitbox_backend::CacheBackend;
use hitbox_redis::RedisBackend;

use crate::FutureAdapter;

pub enum CacheServiceState {
    Running,
    Stopped,
}

pub struct Cache<B> {
    state: CacheServiceState,
    backend: B,
}

impl<B> Cache<B> {
    fn builder() -> CacheBuilder<RedisBackend> {
        CacheBuilder { backend: None }
    }

    async fn start(&self) {

    }

    async fn process<F, Req, Res, ResFuture>(&self, upstream: F, request: Req) -> Res
    where
        Req: Send + Sync,
        F: Fn(Req) -> ResFuture + Send + Sync,
        ResFuture: Future<Output=Res> + Send + Sync,
        Res: Send + Sync + CacheableResponse + std::fmt::Debug,
        B: CacheBackend + Send + Sync,
    {
        let adapter_result = FutureAdapter::new(upstream, request, &self.backend);
        // let settings = self.settings.clone();
        let settings = CacheSettings { cache: Status::Enabled, lock: Status::Disabled, stale: Status::Disabled };
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
            backend: Some(backend)
        }
    }

    fn build(self) -> Cache<B> {
        Cache { state: CacheServiceState::Stopped, backend: self.backend.unwrap()}
    }
}
