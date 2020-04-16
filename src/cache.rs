use std::boxed::Box;

use actix::{
    dev::{MessageResponse, ToEnvelope},
    Actor, Addr, Handler, Message, ResponseFuture,
};
use actix_cache_redis::actor::Set;
use actix_cache_backend::Get;
use log::warn;
use serde::{Serialize, Deserialize};

use crate::actor::Cache;

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
        self.cache_ttl() - 5
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
    type Result = Result<<M as Message>::Result, actix::MailboxError>;
}

use serde::de::DeserializeOwned;

impl<'a, A, M> Handler<QueryCache<A, M>> for Cache
where
    A: Actor + Handler<M> + Send,
    M: Message + Cacheable + Send + 'static,
    M::Result: MessageResponse<A, M> + Serialize + std::fmt::Debug + DeserializeOwned + Send,
    <A as Actor>::Context: ToEnvelope<A, M>,
{
    type Result = ResponseFuture<Result<<M as Message>::Result, actix::MailboxError>>;

    fn handle(&mut self, msg: QueryCache<A, M>, _: &mut Self::Context) -> Self::Result {
        let backend = self.backend.clone();
        let key = msg.message.cache_key();
        let res = async move { 
            let cached = CachedValue::retrive(&backend, &msg).await;
            match cached {
                Some(res) => Ok(res.into_inner()),
                None => {
                    let res = msg.upstream.send(msg.message).await?;
                    let cached = CachedValue::new(res, 10);
                    backend.send(Set { 
                        value: serde_json::to_string(&cached).unwrap(), 
                        key,
                        ttl: None 
                    }).await.unwrap().unwrap();
                    Ok(cached.into_inner())
                }
            }
        };
        Box::pin(res)
    }
}

use actix_cache_redis::actor::RedisActor;
use chrono::{DateTime, Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
struct CachedValue<T> {
    pub data: T,
    pub expired: DateTime<Utc>,
}


impl<T> CachedValue<T> {
    pub fn new(data: T, stale: i64) -> Self
    where
        T: Serialize,
    {
        CachedValue {
            data,
            expired: Utc::now() + Duration::seconds(stale),
        }
    }

    async fn retrive<A, M>(backend: &Addr<RedisActor>, msg: &QueryCache<A, M>) -> Option<Self>
    where
        A: Actor,
        M: Message + Cacheable + Send,
        M::Result: MessageResponse<A, M> + Send + 'static,
        T: DeserializeOwned
    {
        let value = backend
            .send(Get { key: msg.message.cache_key() })
            .await;
        let serialized = match value {
            Ok(Ok(value)) => value,
            Ok(Err(_error)) => {
                // warn!("Cache backend error: {}", error);
                warn!("Cache backend error.");
                None
            },
            Err(error) => {
                warn!("Actix error: {}", error);
                None
            }
        };
        serialized
            .map(|data| 
                serde_json::from_str(&data)
                    .map_err(|err| {
                        warn!("Cache data deserializtion error: {}", err);
                        err
                    })
                    .ok()
            )
            .flatten()
    }

    /// Return instance of inner type.
    pub fn into_inner(self) -> T {
        self.data
    }
}
