use axum::async_trait;
use hitbox_backend::{
    serializer::{JsonSerializer, Serializer},
    BackendError, BackendResult, CacheBackend, CacheableResponse, CachedValue, DeleteStatus,
};
use hitbox_tower::Cache;
use hyper::{Body, Server};
use std::{convert::Infallible, net::SocketAddr};
use stretto::AsyncCache;

use http::{Request, Response};
use tower::make::Shared;

async fn handle(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, World!".into()))
}

#[derive(Clone)]
struct InMemoryBackend {
    cache: AsyncCache<String, Vec<u8>>,
}

impl InMemoryBackend {
    fn new() -> Self {
        Self {
            cache: AsyncCache::new(12960, 1e6 as i64, tokio::spawn).unwrap(),
        }
    }
}

#[async_trait]
impl CacheBackend for InMemoryBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        match self.cache.get(&key).await {
            Some(cached) => Ok(Some(
                JsonSerializer::<Vec<u8>>::deserialize(cached.value().to_owned())
                    .map_err(BackendError::from)
                    .unwrap(),
            )),
            None => Ok(None),
        }
    }

    async fn set<T>(
        &self,
        key: String,
        value: CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        <T as CacheableResponse>::Cached: serde::Serialize + Send,
    {
        let serialized =
            JsonSerializer::<Vec<u8>>::serialize(&value).map_err(BackendError::from)?;
        self.cache.insert(key, serialized, 42).await;
        Ok(())
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        unimplemented!()
    }

    async fn start(&self) -> BackendResult<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let inmemory = InMemoryBackend::new();
    let service = tower::ServiceBuilder::new()
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(Cache::builder().backend(inmemory).build())
        .service_fn(handle);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    Server::bind(&addr)
        .serve(Shared::new(service))
        .await
        .expect("server error");
}
