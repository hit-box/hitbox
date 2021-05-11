use actix::{dev::MessageResponse, prelude::*};
use hitbox::{CacheError, Cacheable};

pub trait IntoCache: Cacheable {
    /// Helper method to convert Message into [QueryCache] message.
    ///
    /// # Examples
    /// ```
    /// use actix::prelude::*;
    /// use hitbox_actix::prelude::*;
    /// use serde::Serialize;
    ///
    /// struct Upstream;
    ///
    /// impl Actor for Upstream {
    ///     type Context = Context<Self>;
    /// }
    ///
    /// #[derive(Cacheable, Serialize, Message, Debug, PartialEq)]
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

impl<M: Message + Cacheable> IntoCache for M {}

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
    pub upstream: Addr<A>,
    pub message: M,
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
    /// This method compose final cache key from Cacheable::cache_key
    /// and Upstream actor type name.
    pub fn cache_key(&self) -> Result<String, CacheError> {
        Ok(format!(
            "{}::{}",
            self.upstream_name(),
            self.message.cache_key()?
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
