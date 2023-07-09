use std::future::Future;
use std::marker::PhantomData;

use async_trait::async_trait;
use fred::{
    clients::RedisClient,
    interfaces::{ClientLike, KeysInterface},
    types::{Expiration, FromRedis, RedisKey, RedisValue},
};
use hitbox_backend::{
    serializer::Serializer, BackendError, BackendResult, CacheBackend, CacheableResponse,
    CachedValue, DeleteStatus,
};

use crate::error::Error;

#[derive(Clone)]
pub struct RedisBackend<S> {
    pub(super) client: RedisClient,
    pub(super) _ser: PhantomData<S>,
}

impl<S> RedisBackend<S> {
    async fn execute<'a, T, Fut, F>(&'a self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&'a RedisClient) -> Fut,
        Fut: Future<Output = Result<T, Error>>,
        T: Send,
    {
        let connection_task = self.client.connect();
        self.client.wait_for_connect().await?;
        let client = &self.client;
        let result = f(client).await?;
        self.client.quit().await?;
        connection_task.await??;
        Ok(result)
    }
}

#[async_trait]
impl<S> CacheBackend for RedisBackend<S>
where
    S: Serializer + Send + Sync,
    S::Raw: Send + Sync + FromRedis,
    RedisValue: From<S::Raw>,
{
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        tracing::debug!("RedisBackend::get::{}", &key);
        let key = RedisKey::from(key);
        let result = self
            .execute(|client| async move {
                client
                    .get::<Option<S::Raw>, _>(key)
                    .await
                    .map_err(Error::from)
            })
            .await
            .map_err(BackendError::from)?;
        result
            .map(|value| S::deserialize(value).map_err(BackendError::from))
            .transpose()
    }

    async fn set<T>(
        &self,
        key: String,
        value: &CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        <T as CacheableResponse>::Cached: serde::Serialize + Send + Sync,
    {
        tracing::debug!("RedisBackend::set::{}", &key);
        let key = RedisKey::from(key);
        let ser_value = S::serialize(value).map_err(BackendError::from)?;
        self.execute(|client| async move {
            let expire = ttl.map(|ttl| Expiration::EX(ttl as i64));
            client
                .set::<(), _, S::Raw>(key, ser_value, expire, None, false)
                .await
                .map_err(Error::from)
        })
        .await
        .map_err(BackendError::from)
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        tracing::debug!("RedisBackend::delete::{}", &key);
        let key = RedisKey::from(key);
        self.execute(|client| async move {
            client
                .del::<u32, _>(key)
                .await
                .map(|res| {
                    if res > 0 {
                        DeleteStatus::Deleted(res)
                    } else {
                        DeleteStatus::Missing
                    }
                })
                .map_err(Error::from)
        })
        .await
        .map_err(BackendError::from)
    }

    async fn start(&self) -> BackendResult<()> {
        tracing::debug!("RedisBackend::start");
        Ok(())
    }
}
