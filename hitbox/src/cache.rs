//! Cacheable trait and implementation of cache logic.
use std::boxed::Box;

use actix::{
    dev::{MessageResponse, ToEnvelope},
    Actor, Addr, Handler, Message, ResponseFuture,
};
use serde::{de::DeserializeOwned, Serialize};

use hitbox_backend::{Backend, Delete, Get, Lock, Set};
#[cfg(feature = "derive")]
pub use hitbox_derive::Cacheable;

#[cfg(feature = "metrics")]
use crate::metrics::{
    CACHE_HIT_COUNTER, CACHE_MISS_COUNTER, CACHE_STALE_COUNTER, CACHE_UPSTREAM_HANDLING_HISTOGRAM,
};
use crate::response::CacheableResponse;
use crate::settings::InitialCacheSettings;
use crate::states::initial::InitialState;
use crate::transition_groups::{only_cache, stale, upstream};
use crate::CacheError;

/// Trait describe cache configuration per message type for actix Cache actor.
pub trait Cacheable {
    /// Method should return unique identifier for struct object.
    ///
    /// In cache storage it prepends with cache version and Upstream actor name.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox::cache::Cacheable;
    /// use hitbox::CacheError;
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

    // /// Helper method to convert Message into [QueryCache] message.
    // ///
    // /// # Examples
    // /// ```
    // /// use actix::prelude::*;
    // /// use actix_derive::Message;
    // /// use hitbox::cache::{Cacheable, QueryCache};
    // /// use hitbox::CacheError;
    // /// use serde::Serialize;
    // ///
    // /// struct Upstream;
    // ///
    // /// impl Actor for Upstream {
    // ///     type Context = Context<Self>;
    // /// }
    // ///
    // /// #[derive(Cacheable, Serialize, Message, Debug, Clone, PartialEq)]
    // /// #[rtype(result = "()")]
    // /// struct QueryNothing {
    // ///     id: Option<i32>,
    // /// }
    // ///
    // /// #[actix_rt::main]
    // /// async fn main() {
    // ///     let upstream = Upstream.start();
    // ///     let query = QueryNothing { id: Some(1) }
    // ///         .into_cache(&upstream);
    // /// }
    // /// ```
    // ///
    // /// [QueryCache]: struct.QueryCache.html
    // fn into_cache<A>(self, upstream: &Addr<A>) -> QueryCache<A, Self>
    // where
    // A: Actor,
    // Self: Message + Send + Sized,
    // Self::Result: MessageResponse<A, Self> + Send + 'static,
    // {
    // QueryCache {
    // upstream: upstream.clone(),
    // message: self,
    // }
    // }
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
