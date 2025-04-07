use std::{future::Future, sync::Arc, time::Duration};

use async_trait::async_trait;
use hitbox_core::{CacheKey, CacheValue, CacheableResponse};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    serializer::{Format, Raw, SerializerError},
    BackendError, DeleteStatus,
};

pub type BackendResult<T> = Result<T, BackendError>;

#[async_trait]
pub trait Backend: Sync + Send {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>>;

    async fn write(
        &self,
        key: &CacheKey,
        value: CacheValue<Raw>,
        ttl: Option<Duration>,
    ) -> BackendResult<()>;

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus>;

    fn format(&self) -> &Format {
        &Format::Json
    }
}

#[async_trait]
impl<T: Backend> CacheBackend for T {}

#[async_trait]
impl Backend for &dyn Backend {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        (*self).read(key).await
    }

    async fn write(
        &self,
        key: &CacheKey,
        value: CacheValue<Raw>,
        ttl: Option<Duration>,
    ) -> BackendResult<()> {
        (*self).write(key, value, ttl).await
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        (*self).delete(key).await
    }

    fn format(&self) -> &Format {
        (*self).format()
    }
}

#[async_trait]
impl Backend for Box<dyn Backend> {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        (**self).read(key).await
    }

    async fn write(
        &self,
        key: &CacheKey,
        value: CacheValue<Raw>,
        ttl: Option<Duration>,
    ) -> BackendResult<()> {
        (**self).write(key, value, ttl).await
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        (**self).remove(key).await
    }

    fn format(&self) -> &Format {
        (**self).format()
    }
}

#[async_trait]
impl Backend for Arc<dyn Backend> {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        (**self).read(key).await
    }

    async fn write(
        &self,
        key: &CacheKey,
        value: CacheValue<Raw>,
        ttl: Option<Duration>,
    ) -> BackendResult<()> {
        (**self).write(key, value, ttl).await
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        (**self).remove(key).await
    }

    fn format(&self) -> &Format {
        (**self).format()
    }
}

// pub trait Backend: Send {
//     fn read(
//         &self,
//         key: &CacheKey,
//     ) -> impl Future<Output = BackendResult<Option<CacheValue<Raw>>>> + Send;
//
//     fn write(
//         &self,
//         key: &CacheKey,
//         value: CacheValue<Raw>,
//         ttl: Option<Duration>,
//     ) -> impl Future<Output = BackendResult<()>> + Send;
//
//     fn remove(&self, key: &CacheKey) -> impl Future<Output = BackendResult<DeleteStatus>> + Send;
// }

pub trait CacheBackend: Backend {
    fn get<T>(
        &self,
        key: &CacheKey,
    ) -> impl Future<Output = BackendResult<Option<CacheValue<T::Cached>>>> + Send
    where
        T: CacheableResponse,
        T::Cached: DeserializeOwned,
    {
        async move {
            Ok(self
                .read(key)
                .await?
                .map(|value| {
                    let (meta, value) = value.into_parts();
                    self.format()
                        .deserialize(&value)
                        .map(|value| CacheValue::new(value, meta.expire, meta.stale))
                })
                .transpose()?)
        }
    }

    fn set<T>(
        &self,
        key: &CacheKey,
        value: &CacheValue<T::Cached>,
        ttl: Option<Duration>,
    ) -> impl Future<Output = BackendResult<()>> + Send
    where
        T: CacheableResponse,
        T::Cached: Serialize + Send + Sync,
    {
        async move {
            let serialized_value = self.format().serialize(&value.data)?;
            self.write(
                key,
                CacheValue::new(serialized_value, value.expire, value.stale),
                ttl,
            )
            .await
        }
    }

    fn delete(&self, key: &CacheKey) -> impl Future<Output = BackendResult<DeleteStatus>> + Send {
        async move { self.remove(key).await }
    }
}
