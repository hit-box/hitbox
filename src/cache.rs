use serde::de::DeserializeOwned;
use std::boxed::Box;

use actix::{
    dev::{MessageResponse, ToEnvelope},
    Actor, Addr, Handler, Message, ResponseFuture,
};
use actix_cache_backend::{Get, Set, Lock, LockStatus};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::actor::Cache;
use crate::CacheError;

#[cfg(feature = "derive")]
pub use actix_cache_derive::Cacheable;

/// Trait describe cache configuration per message for actix Cache actor.
pub trait Cacheable {
    /// Method should return unique identifier for struct object.
    ///
    /// Format of key describes as:
    /// `{actor}::{message type}::{message attributes}`
    ///
    /// In cache storage it prepends with cache version.
    ///
    /// # Examples
    ///
    /// ```
    /// use actix_cache::cache::Cacheable;
    /// use actix_cache::CacheError;
    ///
    /// struct QueryNothing {
    ///     id: Option<i32>,
    /// }
    ///
    /// impl Cacheable for QueryNothing {
    ///     fn cache_key(&self) -> Result<String, CacheError> {
    ///         let key = format!("database::QueryNothing::id::{}", self.id.map_or_else(
    ///             || "None".to_owned(), |id| id.to_string())
    ///         );
    ///         Ok(key)
    ///     }
    /// }
    ///
    /// let query = QueryNothing { id: Some(1) };
    /// assert_eq!(query.cache_key().unwrap(), "database::QueryNothing::id::1");
    /// let query = QueryNothing { id: None };
    /// assert_eq!(query.cache_key().unwrap(), "database::QueryNothing::id::None");
    /// ```
    fn cache_key(&self) -> Result<String, CacheError>;

    /// Describe time-to-live (ttl) value for cache storage in seconds.
    ///
    /// After that time value will be removed from cache storage.
    fn cache_ttl(&self) -> u32 {
        60
    }

    /// Describe expire\stale timeout value for cache storage in seconds.
    ///
    /// After that time cached value marked as stale.
    fn cache_stale_ttl(&self) -> u32 {
        let ttl = self.cache_ttl();
        let stale_time = 5;
        if ttl >= stale_time {
            ttl - stale_time
        } else {
            0
        }
    }

    /// Describe current cache version for this message type.
    fn cache_version(&self) -> u32 {
        0
    }

    fn into_cache<A>(self, upstream: Addr<A>) -> QueryCache<A, Self>
    where
        A: Actor,
        Self: Message + Send + Sized,
        Self::Result: MessageResponse<A, Self> + Send + 'static,
    {
        QueryCache {
            upstream,
            message: self,
        }
    }
}

pub struct QueryCache<A, M>
where
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
    A: Actor,
{
    upstream: Addr<A>,
    message: M,
}

impl<'a, A, M> Message for QueryCache<A, M>
where
    A: Actor,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send + Deserialize<'a>,
{
    type Result = Result<<M as Message>::Result, CacheError>;
}

async fn set_value<T, B>(
    value: &CachedValue<T>,
    backend: Addr<B>,
    key: String,
    ttl: Option<u32>,
) -> Result<(), CacheError>
where
    T: Serialize,
    B: Actor + Backend,
    <B as Actor>::Context: ToEnvelope<B, Set>,
{
    let _ = backend
        .send(Set { value: serde_json::to_string(value)?, key, ttl })
        .await?
        .map_err(|error| {
            warn!("Updating cache error: {}", error);
            CacheError::BackendError(error)
        });
    Ok(())
}

impl<'a, A, M, B> Handler<QueryCache<A, M>> for Cache<B>
where
    B: Actor + Backend,
    <B as Actor>::Context: ToEnvelope<B, Get> + ToEnvelope<B, Set> + ToEnvelope<B, Lock>,
    A: Actor + Handler<M> + Send,
    M: Message + Cacheable + Send + 'static,
    M::Result: MessageResponse<A, M> + Serialize + std::fmt::Debug + DeserializeOwned + Send,
    <A as Actor>::Context: ToEnvelope<A, M>,
{
    type Result = ResponseFuture<Result<<M as Message>::Result, CacheError>>;

    fn handle(&mut self, msg: QueryCache<A, M>, _: &mut Self::Context) -> Self::Result {
        let backend = self.backend.clone();
        let (enabled, cache_key) = match msg.message.cache_key() {
            Ok(value) => (self.enabled, value),
            Err(error) => {
                warn!("Creating cache key error: {}", error);
                (false, String::new())
            }
        };
        let res = async move {
            debug!("Try retrieve cached value from backend");
            let cached = if enabled {
                Some(CachedValue::retrieve(&backend, cache_key.clone()).await)
            } else {
                None
            };

            match cached {
                Some(CachedValueState::Actual(res)) => {
                    debug!("Cached value retrieved successfully");
                    Ok(res.into_inner())
                }
                Some(CachedValueState::Stale(res)) => {
                    debug!("Cache is stale, trying to acquire lock.");
                    let key = msg.message.cache_key()?;
                    let ttl = 1;
                    let lock_status = backend
                        .send(Lock { key, ttl }).await?
                        // ToDo: use correct error
                        .map_err(|_| CacheError::DeserializeError)?;
                    match lock_status {
                        LockStatus::Acquired => {
                            debug!("Lock acquired.");
                            let ttl = Some(msg.message.cache_ttl());
                            let cache_stale_ttl = msg.message.cache_stale_ttl();
                            let upstream_result = msg.upstream
                                .send(msg.message)
                                .await?;
                            debug!("Lock acquired.");
                            let cached = CachedValue::new(upstream_result, cache_stale_ttl);
                            set_value(&cached, backend, cache_key, ttl).await?;
                            Ok(cached.into_inner())
                        },
                        LockStatus::Locked => {
                            debug!("Cache locked.");
                            Ok(res.into_inner())
                        },
                    }
                }
                Some(CachedValueState::Miss) => {
                    debug!("Cache miss");
                    let cache_stale_ttl = msg.message.cache_stale_ttl();
                    let ttl = Some(msg.message.cache_ttl());
                    let upstream_result = msg.upstream.send(msg.message).await?;
                    let cached = CachedValue::new(upstream_result, cache_stale_ttl);
                    debug!("Update value in cache");
                    set_value(&cached, backend, cache_key, ttl).await?;
                    Ok(cached.into_inner())
                },
                None => {
                    debug!("Cache disabled");
                    let upstream_result = msg.upstream.send(msg.message).await?;
                    Ok(upstream_result)
                }
            }
        };
        Box::pin(res)
    }
}

use actix_cache_backend::Backend;
use chrono::{DateTime, Duration, Utc};

enum CachedValueState<T> {
    Actual(CachedValue<T>),
    Stale(CachedValue<T>),
    Miss,
}

impl<T> From<Option<CachedValue<T>>> for CachedValueState<T> {
    fn from(cached_value: Option<CachedValue<T>>) -> Self {
        match cached_value {
            Some(value) => {
                if value.expired < Utc::now() {
                    CachedValueState::Stale(value)
                } else {
                    CachedValueState::Actual(value)
                }
            }
            None => CachedValueState::Miss
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedValue<T> {
    pub data: T,
    pub expired: DateTime<Utc>,
}

impl<T> CachedValue<T> {
    pub fn new(data: T, stale: u32) -> Self
    where
        T: Serialize,
    {
        CachedValue {
            data,
            expired: Utc::now() + Duration::seconds(stale as i64),
        }
    }

    async fn retrieve<B>(backend: &Addr<B>, cache_key: String) -> CachedValueState<T>
    where
        B: Backend,
        <B as Actor>::Context: ToEnvelope<B, Get>,
        T: DeserializeOwned,
    {
        let value = backend.send(Get { key: cache_key }).await;
        let serialized = match value {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                warn!("Cache backend error: {}", error);
                None
            }
            Err(error) => {
                warn!("Actix error: {}", error);
                None
            }
        };
        let cached_value: Option<CachedValue<T>> = serialized
            .map(|data| {
                serde_json::from_str(&data)
                    .map_err(|err| {
                        warn!("Cache data deserialization error: {}", err);
                        err
                    })
                    .ok()
            })
            .flatten();
        CachedValueState::from(cached_value)
    }

    /// Return instance of inner type.
    pub fn into_inner(self) -> T {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Message;

    impl Cacheable for Message {
        fn cache_key(&self) -> Result<String, CacheError> {
            Ok("Message".to_owned())
        }

        fn cache_ttl(&self) -> u32 {
            2
        }
    }

    #[test]
    fn test_cache_stale_ttl_subtract_owerflow() {
        let a = Message;
        assert_eq!(0, a.cache_stale_ttl());
    }
}
