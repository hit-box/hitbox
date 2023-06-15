use std::sync::Arc;

use axum::{
    async_trait,
    extract::{Path, Query},
    routing::get,
    Router,
};
use hitbox::{dev::CacheBackend, CachedValue};
use hitbox_backend::{
    response2::CacheableResponse,
    serializer::{JsonSerializer, Serializer},
    BackendError, BackendResult, DeleteStatus,
};
use hitbox_redis::RedisBackend;
use hitbox_tower::Cache;
use http::StatusCode;
use lazy_static::lazy_static;
use stretto::AsyncCache;
use tower::ServiceBuilder;

lazy_static! {
    static ref BACKEND: Arc<RedisBackend> = Arc::new(RedisBackend::new().unwrap());
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

async fn handler(Path(name): Path<String>) -> Result<String, StatusCode> {
    println!("++++++++++++++++++++++++++++++++++++++");
    Ok(format!("Hello, {name}"))
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let backend = RedisBackend::new().unwrap();
    let inmemory = InMemoryBackend::new();
    // build our application with a single route
    let app = Router::new().route("/:name", get(handler)).layer(
        ServiceBuilder::new()
            .layer(Cache::builder().backend(backend).build())
            .layer(
                Cache::builder()
                    // .backend(&BACKEND)
                    .backend(inmemory)
                    .build(),
            ),
    );

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
