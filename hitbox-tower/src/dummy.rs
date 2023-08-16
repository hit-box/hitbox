use axum::async_trait;
use hitbox_backend::{BackendResult, CacheBackend, CacheableResponse, CachedValue, DeleteStatus};

pub struct DummyBackend;

#[async_trait]
impl CacheBackend for DummyBackend {
    async fn get<T>(&self, _key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        unreachable!()
    }

    async fn set<T>(
        &self,
        _key: String,
        _value: &CachedValue<T::Cached>,
        _ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        T::Cached: serde::Serialize + Send + Sync,
    {
        unreachable!()
    }

    async fn delete(&self, _key: String) -> BackendResult<DeleteStatus> {
        unreachable!()
    }

    async fn start(&self) -> BackendResult<()> {
        unreachable!()
    }
}
