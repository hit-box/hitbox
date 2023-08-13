//! Cacheable trait and implementation of cache logic.

use std::sync::Arc;

use crate::{predicates::Predicate, CacheError};
use async_trait::async_trait;
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use hitbox_derive::Cacheable;

pub use hitbox_backend::CacheableResponse;

/// Trait describes cache configuration per type that implements this trait.
#[async_trait]
pub trait Cacheable {
    /// Method should return unique identifier for struct object.
    ///
    /// In cache storage it may prepends with cache version and Upstream name.
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
    ///     fn cache_key(&self) -> Result<String, CacheError> {
    ///         let key = format!("{}::id::{}", self.cache_key_prefix(), self.id.map_or_else(
    ///             || "None".to_owned(), |id| id.to_string())
    ///         );
    ///         Ok(key)
    ///     }
    ///     fn cache_key_prefix(&self) -> String { "database::QueryNothing".to_owned() }
    /// }
    ///
    /// let query = QueryNothing { id: Some(1) };
    /// assert_eq!(query.cache_key().unwrap(), "database::QueryNothing::id::1");
    /// let query = QueryNothing { id: None };
    /// assert_eq!(query.cache_key().unwrap(), "database::QueryNothing::id::None");
    /// ```
    async fn cache_key(&self) -> Result<String, CacheError>;

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
    ///
    /// ```ignore
    /// |__cache_is_valid__|__cache_is_stale__| -> time
    ///                    ^                  ^
    ///                 stale_ttl       ttl (cache evicted)
    /// ```
    fn cache_stale_ttl(&self) -> u32 {
        let ttl = self.cache_ttl();
        let stale_time = 5;
        if ttl >= stale_time {
            ttl - stale_time
        } else {
            0
        }
    }

    /// Describe current cache version for this type.
    fn cache_version(&self) -> u32 {
        0
    }
}

#[derive(Debug)]
pub enum CachePolicy<T> {
    Cacheable { key: CacheKey, request: T },
    NonCacheable(T),
}

#[derive(Debug)]
pub struct CacheKey {
    pub parts: Vec<KeyPart>,
    pub version: u32,
    pub prefix: String,
}

impl CacheKey {
    pub fn serialize(&self) -> String {
        self.parts
            .iter()
            .map(|part| {
                format!(
                    "{}:{}",
                    part.key,
                    part.value.clone().unwrap_or("None".to_owned())
                )
            })
            .collect::<Vec<_>>()
            .join("::")
    }
}

#[derive(Debug)]
pub struct KeyPart {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug)]
pub struct KeyParts<T: Sized> {
    pub subject: T,
    pub parts: Vec<KeyPart>,
}

impl<T> KeyParts<T> {
    pub fn push(&mut self, part: KeyPart) {
        self.parts.push(part)
    }

    pub fn append(&mut self, parts: &mut Vec<KeyPart>) {
        self.parts.append(parts)
    }

    pub fn into_cache_key(self) -> (T, CacheKey) {
        (
            self.subject,
            CacheKey {
                version: 0,
                prefix: String::new(),
                parts: self.parts,
            },
        )
    }
}

#[async_trait]
pub trait Extractor {
    type Subject;
    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject>;
}

#[async_trait]
impl<T> Extractor for Box<T>
where
    T: Extractor + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn get(&self, subject: T::Subject) -> KeyParts<T::Subject> {
        self.as_ref().get(subject).await
    }
}

#[async_trait]
impl<T> Extractor for Arc<T>
where
    T: Extractor + Send + Sync + ?Sized,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn get(&self, subject: T::Subject) -> KeyParts<T::Subject> {
        self.as_ref().get(subject).await
    }
}

#[async_trait]
pub trait CacheableRequest
where
    Self: Sized,
{
    async fn cache_policy<P, E>(self, predicates: P, extractors: E) -> CachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync;
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     struct Message(i32);
//
//     impl Cacheable for Message {
//         fn cache_key(&self) -> Result<String, CacheError> {
//             Ok("Message".to_owned())
//         }
//         fn cache_key_prefix(&self) -> String {
//             "Message".to_owned()
//         }
//         fn cache_ttl(&self) -> u32 {
//             2
//         }
//     }
//
//     #[test]
//     fn test_cache_stale_ttl_subtract_overflow() {
//         let a = Message(42);
//         assert_eq!(0, a.cache_stale_ttl());
//     }
//
//     #[allow(dead_code)]
//     async fn upstream_fn(message: Message) -> i32 {
//         message.0
//     }
// }
