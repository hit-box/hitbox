use actix::Message;

// pub enum CachePolicy<T> {
    // Cacheable(T),
    // NonCacheable,
// }

// pub trait CacheableResponse
// {
    // type Cached;
    // type NonCached;
    // fn cache_policy(&self) -> CachePolicy<&Self::Cached>;
// }

// impl CacheableResponse for i32 {
    // type Cached = i32;
    // type NonCached = ();
    // fn cache_policy(&self) -> CachePolicy<&Self::Cached> {
        // CachePolicy::Cacheable(self)
    // }
// }

// impl<T, E> CacheableResponse for Result<T, E> {
    // type Cached = T;
    // type NonCached = E;
    // fn cache_policy(&self) -> CachePolicy<&Self::Cached> {
        // match self {
            // Ok(value) => CachePolicy::Cacheable(value),
            // Err(_) => CachePolicy::NonCacheable,
        // }
    // }
// }

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::cache::CachedValue;

pub enum CachePolicy<T> {
    Cacheable(T),
    NonCacheable,
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
    fn cache_policy(&self) -> CachePolicy<&Self::Cached>;
    fn serialize(&self) -> Result<String, ResponseError>;
    fn deserialize(data: String) -> Result<Self, ResponseError>;
    fn from_cached(cached: Self::Cached) -> Self;
}

impl CacheableResponse for i32 {
    type Cached = i32;
    fn cache_policy(&self) -> CachePolicy<&Self::Cached> {
        CachePolicy::Cacheable(self)
    }
    fn serialize(&self) -> Result<String, ResponseError> {
        Ok(serde_json::to_string(self)?)
    }
    fn deserialize(data: String) -> Result<Self, ResponseError> {
        Ok(serde_json::from_str(data.as_str())?)
    }
    fn from_cached(cached: Self::Cached) -> Self {
        cached
    }
}

impl<I, E> CacheableResponse for Result<I, E> 
where
    I: Serialize + DeserializeOwned,
{
    type Cached = I;
    fn cache_policy(&self) -> CachePolicy<&Self::Cached> {
        match self {
            Ok(value) => CachePolicy::Cacheable(value),
            Err(error) => CachePolicy::NonCacheable,
        }
    }
    fn serialize(&self) -> Result<String, ResponseError> {
        match self.cache_policy() {
            CachePolicy::Cacheable(value) => Ok(serde_json::to_string(value)?),
            CachePolicy::NonCacheable => Err(ResponseError::NonCacheable),
        }
    }
    fn deserialize(data: String) -> Result<Self, ResponseError> {
        Ok(serde_json::from_str::<I>(data.as_str())
            .map(Result::Ok)?)
    }
    fn from_cached(cached: Self::Cached) -> Self {
        Ok(cached)
    }
}

#[cfg(test)]
mod tests {
    use super::CacheableResponse;
    #[test]
    fn cacheable_response_result_serialize() {
        let res: Result<i32, i32> = Ok(42);
        assert_eq!(res.serialize(), Ok("42".to_owned()));
    }

    #[test]
    fn cacheable_response_result_deserialize() {
        let res = "42".to_owned();
        assert_eq!(Result::<i32, ()>::deserialize(res), Ok(Ok(42)));
    }
}
