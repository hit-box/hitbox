use crate::actor::Cache;
use actix::{
    dev::{MessageResponse, ToEnvelope},
    Actor, Addr, Handler, Message, ResponseFuture,
};
use std::boxed::Box;

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

impl<A, M> Message for QueryCache<A, M>
where
    A: Actor,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
{
    type Result = M::Result;
}

pub struct Cached<T>(pub T);

impl<A, M, I, E> Handler<QueryCache<A, M>> for Cache
where
    A: Actor + Handler<M> + Send,
    I: 'static,
    E: 'static,
    M: Message<Result = Result<I, E>> + Cacheable + Send + 'static,
    M::Result: Send,
    M::Result: MessageResponse<A, M> + Send,
    <A as Actor>::Context: ToEnvelope<A, M>,
{
    type Result = ResponseFuture<<M as Message>::Result>;

    fn handle(&mut self, msg: QueryCache<A, M>, _: &mut Self::Context) -> Self::Result {
        log::warn!("YOHOOOO");
        let res = async { msg.upstream.send(msg.message).await.unwrap() };
        Box::pin(res)
    }
}

pub struct Test;

use actix::prelude::*;

impl Actor for Test {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        log::info!("Cache actor started");
    }
}

#[derive(Message)]
#[rtype(result = "Result<i32, ()>")]
pub struct Ping;

impl Cacheable for Ping {
    fn cache_key(&self) -> String {
        "Ping::".to_owned()
    }
}

impl Handler<Ping> for Test {
    type Result = Result<i32, ()>;

    fn handle(&mut self, _msg: Ping, _: &mut Self::Context) -> Self::Result {
        Ok(42)
    }
}

#[derive(Message)]
#[rtype(result = "i32")]
pub struct Pong;

impl Cacheable for Pong {
    fn cache_key(&self) -> String {
        "Pong::".to_owned()
    }
}

impl Handler<Pong> for Test {
    type Result = i32;

    fn handle(&mut self, _msg: Pong, _: &mut Self::Context) -> Self::Result {
        42
    }
}

struct SyncTest;

impl Actor for SyncTest {
    type Context = SyncContext<Self>;
}

impl Handler<Ping> for SyncTest {
    type Result = Result<i32, ()>;

    fn handle(&mut self, _msg: Ping, _: &mut Self::Context) -> Self::Result {
        Ok(42)
    }
}

#[cfg(test)]
mod tests {
    use crate::cache::*;

    #[actix_rt::test]
    async fn test() {
        let synctest = SyncArbiter::start(10, move || SyncTest {});
        let cache = Cache {}.start();
        let test = Test {}.start();
        let res = cache.send(Ping {}.into_cache(test.clone())).await.unwrap();
        assert_eq!(res, Ok(42));
        // let res = cache.send(QueryCache { message: Pong {}, upstream: test}).await.unwrap();
        // assert_eq!(res, 42);
        let res = cache.send(Ping {}.into_cache(synctest)).await.unwrap();
        assert_eq!(res, Ok(42));
    }
}
