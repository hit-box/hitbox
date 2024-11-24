use std::{future::Future, time::Duration};

use async_trait::async_trait;
use hitbox_core::{CacheKey, CacheValue, CacheableResponse};
use serde::{de::DeserializeOwned, Serialize};

use crate::{serializer::SerializerError, BackendError, DeleteStatus};

pub type Raw = Vec<u8>;
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
}

// #[async_trait]
// pub trait CacheBackend: Backend {
//     async fn get<T>(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<T::Cached>>>
//     where
//         T: CacheableResponse,
//         T::Cached: DeserializeOwned,
//     {
//         match self.read(key).await? {
//             Some(value) => {
//                 let (meta, value) = value.into_parts();
//                 let value = serde_json::from_slice(&value)
//                     .map_err(|error| SerializerError::Deserialize(Box::new(error)))?;
//                 Ok(Some(CacheValue::new(value, meta.expire)))
//             }
//             None => Ok(None),
//         }
//     }
//
//     async fn set<T>(
//         &self,
//         key: &CacheKey,
//         value: &CacheValue<T::Cached>,
//         ttl: Option<Duration>,
//     ) -> BackendResult<()>
//     where
//         T: CacheableResponse,
//         T::Cached: Serialize + Send + Sync,
//     {
//         let serialized_value = serde_json::to_vec(&value.data)
//             .map_err(|error| SerializerError::Serialize(Box::new(error)))?;
//         self.write(key, CacheValue::new(serialized_value, value.expire), ttl)
//             .await
//     }
//
//     async fn delete(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
//         self.remove(key).await
//     }
// }

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
            match self.read(key).await? {
                Some(value) => {
                    let (meta, value) = value.into_parts();
                    let value = serde_json::from_slice(&value)
                        .map_err(|error| SerializerError::Deserialize(Box::new(error)))?;
                    Ok(Some(CacheValue::new(value, meta.expire)))
                }
                None => Ok(None),
            }
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
            // let (meta, value) = value.into_parts();
            let serialized_value = serde_json::to_vec(&value.data)
                .map_err(|error| SerializerError::Serialize(Box::new(error)))?;
            self.write(key, CacheValue::new(serialized_value, value.expire), ttl)
                .await
        }
    }

    fn delete(&self, key: &CacheKey) -> impl Future<Output = BackendResult<DeleteStatus>> + Send {
        async move { self.remove(key).await }
    }
}
