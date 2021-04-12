//! Cacheable trait and implementation of cache logic.
use std::boxed::Box;

use actix::{
    dev::{MessageResponse, ToEnvelope},
    Actor, Addr, Handler, Message, ResponseFuture,
};
use actix_cache_backend::{Backend, Delete, Get, Lock, LockStatus, Set};
#[cfg(feature = "derive")]
pub use actix_cache_derive::Cacheable;
use chrono::{DateTime, Duration, Utc};
use log::{debug, warn, info};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[cfg(feature = "metrics")]
use crate::metrics::{
    CACHE_HIT_COUNTER, CACHE_MISS_COUNTER, CACHE_STALE_COUNTER, CACHE_UPSTREAM_HANDLING_HISTOGRAM,
};
use crate::CacheError;
use crate::actor;

/// Trait describe cache configuration per message type for actix Cache actor.
pub trait Cacheable {
    /// Method should return unique identifier for struct object.
    ///
    /// In cache storage it prepends with cache version and Upstream actor name.
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
    ///     fn cache_message_key(&self) -> Result<String, CacheError> {
    ///         let key = format!("{}::id::{}", self.cache_key_prefix(), self.id.map_or_else(
    ///             || "None".to_owned(), |id| id.to_string())
    ///         );
    ///         Ok(key)
    ///     }
    ///     fn cache_key_prefix(&self) -> String { "database::QueryNothing".to_owned() }
    /// }
    ///
    /// let query = QueryNothing { id: Some(1) };
    /// assert_eq!(query.cache_message_key().unwrap(), "database::QueryNothing::id::1");
    /// let query = QueryNothing { id: None };
    /// assert_eq!(query.cache_message_key().unwrap(), "database::QueryNothing::id::None");
    /// ```
    fn cache_message_key(&self) -> Result<String, CacheError>;

    /// Method return cache key prefix based on message type.
    fn cache_key_prefix(&self) -> String;

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

    /// Helper method to convert Message into [QueryCache] message.
    ///
    /// # Examples
    /// ```
    /// use actix::prelude::*;
    /// use actix_derive::Message;
    /// use actix_cache::cache::{Cacheable, QueryCache};
    /// use actix_cache::CacheError;
    /// use serde::Serialize;
    ///
    /// struct Upstream;
    ///
    /// impl Actor for Upstream {
    ///     type Context = Context<Self>;
    /// }
    ///
    /// #[derive(Cacheable, Serialize, Message, Debug, Clone, PartialEq)]
    /// #[rtype(result = "()")]
    /// struct QueryNothing {
    ///     id: Option<i32>,
    /// }
    ///
    /// #[actix_rt::main]
    /// async fn main() {
    ///     let upstream = Upstream.start();
    ///     let query = QueryNothing { id: Some(1) }
    ///         .into_cache(&upstream);
    /// }
    /// ```
    ///
    /// [QueryCache]: struct.QueryCache.html
    fn into_cache<A>(self, upstream: &Addr<A>) -> QueryCache<A, Self>
    where
        A: Actor,
        Self: Message + Send + Sized,
        Self::Result: MessageResponse<A, Self> + Send + 'static,
    {
        QueryCache {
            upstream: upstream.clone(),
            message: self,
        }
    }
}

/// Intermediate actix message which handled by Cache actor.
///
/// This message a product of upstream message and upstream actor address.
/// In other words, QueryCache is a struct that includes base message with user data
/// and address of an actor that is a recipient of this message.
/// You can only send QueryCache messages to Cache actor.
pub struct QueryCache<A, M>
where
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
    A: Actor,
{
    pub(crate) upstream: Addr<A>,
    pub(crate) message: M,
}

impl<A, M> QueryCache<A, M>
where
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
    A: Actor,
{
    /// Returns upstream actor type name or <Unknown>.
    pub(crate) fn upstream_name(&self) -> &'static str {
        std::any::type_name::<A>()
            .rsplit("::")
            .next()
            .unwrap_or("<Unknown>")
    }

    /// Returns final cache key.
    ///
    /// This method compose final cache key from Cacheable::cache_message_key
    /// and Upstream actor type name.
    pub fn cache_key(&self) -> Result<String, CacheError> {
        Ok(format!(
            "{}::{}",
            self.upstream_name(),
            self.message.cache_message_key()?
        ))
    }
}

impl<'a, A, M> Message for QueryCache<A, M>
where
    A: Actor,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
{
    type Result = Result<<M as Message>::Result, CacheError>;
}

use crate::response::CacheableResponse;
use std::fmt::Debug;
use crate::settings::InitialCacheSettings;
use crate::adapted::actix_runtime_adapter::ActixAdapter;
use crate::states::initial::{InitialState, UpstreamPolled};

impl<'a, A, M, B> Handler<QueryCache<A, M>> for actor::CacheActor<B>
where
    B: Actor + Backend,
    <B as Actor>::Context:
        ToEnvelope<B, Get> + ToEnvelope<B, Set> + ToEnvelope<B, Lock> + ToEnvelope<B, Delete>,
    A: Actor + Handler<M> + Send,
    M: Message + Cacheable + Send + 'static + Clone,
    M::Result: MessageResponse<A, M> + CacheableResponse + std::fmt::Debug + Send,
    <<M as actix::Message>::Result as CacheableResponse>::Cached: Serialize + DeserializeOwned,
    <A as Actor>::Context: ToEnvelope<A, M>,
{
    type Result = ResponseFuture<Result<<M as Message>::Result, CacheError>>;

    fn handle(&mut self, msg: QueryCache<A, M>, _: &mut Self::Context) -> Self::Result {
        let adapter = ActixAdapter::new(msg, self.backend.clone());  // @TODO: remove clone
        let initial_state = InitialState { adapter };
        Box::pin(async move {
            let finish = match initial_state.poll_upstream().await {
                UpstreamPolled::Successful(state) => state.finish(),
                _ => unreachable!()
            };
            Ok(finish.result())
        })
    }
}

enum CachedValueState<T> {
    Actual(T),
    Stale(T),
    Miss,
}

impl<T, U> From<Option<CachedValue<T>>> for CachedValueState<U> 
where
    U: CacheableResponse<Cached = T>,
{
    fn from(cached_value: Option<CachedValue<T>>) -> Self {
        match cached_value {
            Some(value) => {
                if value.expired < Utc::now() {
                    CachedValueState::Stale(<U as CacheableResponse>::from_cached(value.into_inner()))
                } else {
                    CachedValueState::Actual(<U as CacheableResponse>::from_cached(value.into_inner()))
                }
            }
            None => CachedValueState::Miss,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CachedValue<T> {
    pub data: T,
    pub expired: DateTime<Utc>,
}

impl<T> CachedValue<T> {
    pub fn new(data: T, stale: u32) -> Self
    where
        T: Serialize + DeserializeOwned,
    {
        CachedValue {
            data,
            expired: Utc::now() + Duration::seconds(stale as i64),
        }
    }

    async fn retrieve<B>(backend: &Addr<B>, cache_key: &str) -> Option<CachedValue<T>>
    where
        B: Backend,
        <B as Actor>::Context: ToEnvelope<B, Get>,
        T: DeserializeOwned,
    {
        let value = backend.send(Get { key: cache_key.to_owned() }).await;
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
                serde_json::from_slice(&data[..])
                    .map_err(|err| {
                        warn!("Cache data deserialization error: {}", err);
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

    /// Store inner value into backend.
    pub(crate) async fn store<B>(
        &self,
        backend: &Addr<B>,
        key: String,
        ttl: Option<u32>,
    ) -> Result<(), CacheError>
    where
        B: Actor + Backend,
        <B as Actor>::Context: ToEnvelope<B, Set>,
        T: Serialize,
    {
        let _ = backend
            .send(Set {
                value: serde_json::to_vec(self)?,
                key,
                ttl,
            })
            .await?
            .map_err(|error| {
                warn!("Updating cache error: {}", error);
                CacheError::BackendError(error)
            });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Message;

    impl Cacheable for Message {
        fn cache_message_key(&self) -> Result<String, CacheError> {
            Ok("Message".to_owned())
        }
        fn cache_key_prefix(&self) -> String {
            "Message".to_owned()
        }
        fn cache_ttl(&self) -> u32 {
            2
        }
    }

    #[test]
    fn test_cache_stale_ttl_subtract_overflow() {
        let a = Message;
        assert_eq!(0, a.cache_stale_ttl());
    }
}
