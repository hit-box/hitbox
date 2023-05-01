//! Trait and datatypes that describes which data should be store in cache.
//!
//! For more detailed information and examples please see [CacheableResponse
//! documentation](trait.CacheableResponse.html).
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "derive")]
pub use hitbox_derive::CacheableResponse;

/// This trait determines which types should be cached or not.
pub enum CachePolicy<T, U> {
    /// This variant should be stored in cache backend
    Cacheable(T),
    /// This variant shouldn't be stored in the cache backend.
    NonCacheable(U),
}

/// This is one of the basic trait which determines should data store in cache backend or not.
///
/// For primitive types and for user-defined types (with derive macro)
/// cache_policy returns CachePolicy::Cached variant.
///
/// For `Result<T, E>` cache_policy method return `CachePolicy::Cacheable(T)` only for data included into
/// `Ok(T)` variant.
///
/// `Option<T>` is the same with Result: for `Some(T)` returns `CachedPolicy::Cacheable(T)`. `None` are
/// `NonCacheable` by default.
///
/// ## User defined types:
/// If you want describe custom caching rules for your own types (for example Enum) you should
/// implement `CacheableResponse` for that type:
///
/// ```rust,ignore
/// use hitbox::{CacheableResponse, CachePolicy};
///
/// enum HttpResponse {
///     Ok(String),
///     Unauthorized(i32),
/// }
///
/// impl CacheableResponse for HttpResponse {
///     type Cached = String;
///     fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
///         match self {
///             HttpResponse::Ok(body) => CachePolicy::Cacheable(body),
///             _ => CachePolicy::NonCacheable(()),
///         }
///     }
///     fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
///         match self {
///             HttpResponse::Ok(body) => CachePolicy::Cacheable(body),
///             _ => CachePolicy::NonCacheable(self),
///         }
///     }
///     fn from_cached(cached: Self::Cached) -> Self {
///         HttpResponse::Ok(cached)
///     }
/// }
/// ```
/// In that case only `HttpResponse::Ok` variant will be saved into the cache backend.
/// And all `String`s from the cache backend will be treated as `HttpReponse::Ok(String)` variant.
pub trait CacheableResponse
where
    Self: Sized,
    Self::Cached: Serialize,
{
    /// Describes what type will be stored into the cache backend.
    type Cached;
    /// Returns cache policy for current type with borrowed data.
    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()>;
    /// Returns cache policy for current type with owned data.
    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self>;
    /// Describes how previously cached data will be transformed into the original type.
    fn from_cached(cached: Self::Cached) -> Self;
}

// There are several CacheableResponse implementations for the most common types.
impl<I, E> CacheableResponse for Result<I, E>
where
    I: CacheableResponse,
{
    type Cached = I::Cached;
    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        match self {
            Ok(value) => match value.into_cache_policy() {
                CachePolicy::Cacheable(value) => CachePolicy::Cacheable(value),
                CachePolicy::NonCacheable(value) => CachePolicy::NonCacheable(Ok(value)),
            },
            Err(_) => CachePolicy::NonCacheable(self),
        }
    }
    fn from_cached(cached: Self::Cached) -> Self {
        Ok(I::from_cached(cached))
    }
    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        match self {
            Ok(value) => match value.cache_policy() {
                CachePolicy::Cacheable(value) => CachePolicy::Cacheable(value),
                CachePolicy::NonCacheable(_) => CachePolicy::NonCacheable(()),
            },
            Err(_) => CachePolicy::NonCacheable(()),
        }
    }
}

/// Implementation `CacheableResponse` for `Result` type.
/// We store to cache only `Ok` variant.
// impl<I, E> CacheableResponse for Result<I, E>
// where
// I: Serialize + DeserializeOwned,
// {
// type Cached = I;
// fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
// match self {
// Ok(value) => CachePolicy::Cacheable(value),
// Err(_) => CachePolicy::NonCacheable(self),
// }
// }
// fn from_cached(cached: Self::Cached) -> Self {
// Ok(cached)
// }
// fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
// match self {
// Ok(value) => CachePolicy::Cacheable(value),
// Err(_) => CachePolicy::NonCacheable(()),
// }
// }
// }

/// Implementation `CacheableResponse` for `Option` type.
/// We store to cache only `Some` variant.
impl<I> CacheableResponse for Option<I>
where
    I: Serialize + DeserializeOwned,
{
    type Cached = I;
    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        match self {
            Some(value) => CachePolicy::Cacheable(value),
            None => CachePolicy::NonCacheable(self),
        }
    }
    fn from_cached(cached: Self::Cached) -> Self {
        Some(cached)
    }
    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        match self {
            Some(value) => CachePolicy::Cacheable(value),
            None => CachePolicy::NonCacheable(()),
        }
    }
}

/// Implementation `CacheableResponse` for primitive types.
macro_rules! CACHEABLE_RESPONSE_IMPL {
    ($type:ty) => {
        impl CacheableResponse for $type {
            type Cached = $type;
            fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
                CachePolicy::Cacheable(self)
            }
            fn from_cached(cached: Self::Cached) -> Self {
                cached
            }
            fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
                CachePolicy::Cacheable(self)
            }
        }
    };
}

CACHEABLE_RESPONSE_IMPL!(());
CACHEABLE_RESPONSE_IMPL!(u8);
CACHEABLE_RESPONSE_IMPL!(u16);
CACHEABLE_RESPONSE_IMPL!(u32);
CACHEABLE_RESPONSE_IMPL!(u64);
CACHEABLE_RESPONSE_IMPL!(usize);
CACHEABLE_RESPONSE_IMPL!(i8);
CACHEABLE_RESPONSE_IMPL!(i16);
CACHEABLE_RESPONSE_IMPL!(i32);
CACHEABLE_RESPONSE_IMPL!(i64);
CACHEABLE_RESPONSE_IMPL!(isize);
CACHEABLE_RESPONSE_IMPL!(f32);
CACHEABLE_RESPONSE_IMPL!(f64);
CACHEABLE_RESPONSE_IMPL!(String);
CACHEABLE_RESPONSE_IMPL!(&'static str);
CACHEABLE_RESPONSE_IMPL!(bool);
