use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[cfg(feature = "derive")]
pub use hitbox_derive::CacheableResponse;

pub enum CachePolicy<T, U> {
    Cacheable(T),
    NonCacheable(U),
}

#[derive(Error, Debug)]
pub enum ResponseError {
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("Value is non cacheable")]
    NonCacheable,
}

pub trait CacheableResponse
where
    Self: Sized,
    Self::Cached: Serialize,
{
    type Cached;
    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self>;
    fn from_cached(cached: Self::Cached) -> Self;
    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()>;
}

// There are several CacheableResponse implementations for the most common types.

/// Implementation `CacheableResponse` for `Result` type.
/// We store to cache only `Ok` variant.
impl<I, E> CacheableResponse for Result<I, E>
where
    I: Serialize + DeserializeOwned,
{
    type Cached = I;
    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        match self {
            Ok(value) => CachePolicy::Cacheable(value),
            Err(_) => CachePolicy::NonCacheable(self),
        }
    }
    fn from_cached(cached: Self::Cached) -> Self {
        Ok(cached)
    }
    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        match self {
            Ok(value) => CachePolicy::Cacheable(value),
            Err(_) => CachePolicy::NonCacheable(()),
        }
    }
}

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
CACHEABLE_RESPONSE_IMPL!(bool);
