use serde::de::DeserializeOwned;
use std::boxed::Box;

use actix::{
    dev::{MessageResponse, ToEnvelope},
    Actor, Addr, Handler, Message, ResponseFuture,
};
use actix_cache_backend::{Get, Set};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::actor::Cache;
use crate::CacheError;

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
    ///
    /// struct QueryNothing {
    ///     id: Option<i32>,
    /// }
    ///
    /// impl Cacheable for QueryNothing {
    ///     fn cache_key(&self) -> String {
    ///         format!("database::QueryNothing::id::{}", self.id.map_or_else(
    ///             || "None".to_owned(), |id| id.to_string())
    ///         )
    ///     }
    /// }
    ///
    /// let query = QueryNothing { id: Some(1) };
    /// assert_eq!(query.cache_key(), "database::QueryNothing::id::1");
    /// let query = QueryNothing { id: None };
    /// assert_eq!(query.cache_key(), "database::QueryNothing::id::None");
    /// ```
    fn cache_key(&self) -> String;

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
    fn cache_version(&self) -> u16 {
        0
    }

    /// Describe cache label.
    ///
    /// By default cache label is a cache key prefix: `{actor}::{message type}`.
    /// This value used for aggregating metrics by actors and message.
    fn label(&self) -> String {
        self.cache_key()
            .split("::")
            .take(2)
            .collect::<Vec<&str>>()
            .join("::")
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

impl<'a, A, M, B> Handler<QueryCache<A, M>> for Cache<B>
where
    B: Actor + Backend,
    <B as Actor>::Context: ToEnvelope<B, Get> + ToEnvelope<B, Set>,
    A: Actor + Handler<M> + Send,
    M: Message + Cacheable + Send + 'static,
    M::Result: MessageResponse<A, M> + Serialize + std::fmt::Debug + DeserializeOwned + Send,
    <A as Actor>::Context: ToEnvelope<A, M>,
{
    type Result = ResponseFuture<Result<<M as Message>::Result, CacheError>>;

    fn handle(&mut self, msg: QueryCache<A, M>, _: &mut Self::Context) -> Self::Result {
        let backend = self.backend.clone();
        let key = msg.message.cache_key();
        let enabled = self.enabled;
        let res = async move {
            debug!("Try retrive cached value from backend");
            let cached = if enabled {
                CachedValue::retrive(&backend, &msg).await
            } else {
                None
            };
            // Maybe we should use cached.state() -> CachedValueState enum
            // and match for this state?
            match cached {
                Some(res) => {
                    debug!("Cached value retrieved successfully");
                    Ok(res.into_inner())
                }
                None => {
                    debug!("Cache miss");
                    let cache_stale_ttl = msg.message.cache_stale_ttl();
                    let cache_ttl = msg.message.cache_ttl();
                    let upstream_result = msg.upstream.send(msg.message).await?;
                    let cached = CachedValue::new(upstream_result, cache_stale_ttl);
                    if enabled {
                        debug!("Update value in cache");
                        let _ = backend
                            .send(Set {
                                value: serde_json::to_string(&cached)?,
                                key,
                                ttl: Some(cache_ttl),
                            })
                            .await?
                            .map_err(|error| {
                                warn!("Updating cache error: {}", error);
                                CacheError::BackendError(error.into())
                            });
                    }
                    Ok(cached.into_inner())
                }
            }
        };
        Box::pin(res)
    }
}

use actix_cache_backend::Backend;
use chrono::{DateTime, Duration, Utc};

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

    async fn retrive<A, B, M>(backend: &Addr<B>, msg: &QueryCache<A, M>) -> Option<Self>
    where
        B: Backend,
        <B as Actor>::Context: ToEnvelope<B, Get>,
        A: Actor,
        M: Message + Cacheable + Send,
        M::Result: MessageResponse<A, M> + Send + 'static,
        T: DeserializeOwned,
    {
        let value = backend
            .send(Get {
                key: msg.message.cache_key(),
            })
            .await;
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
        serialized
            .map(|data| {
                serde_json::from_str(&data)
                    .map_err(|err| {
                        warn!("Cache data deserializtion error: {}", err);
                        err
                    })
                    .ok()
            })
            .flatten()
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
        fn cache_key(&self) -> String {
            "Message".to_owned()
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
