use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use hitbox_core::CachedValue;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerializerError {
    #[error(transparent)]
    Serialize(Box<dyn std::error::Error + Send>),

    #[error(transparent)]
    Deserialize(Box<dyn std::error::Error + Send>),
}

pub trait Serializer {
    type Raw;

    fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
    where
        T: DeserializeOwned;

    fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
    where
        T: Serialize;
}

#[derive(Deserialize, Serialize)]
pub struct SerializableCachedValue<U> {
    data: U,
    expired: DateTime<Utc>,
}

impl<U> SerializableCachedValue<U> {
    pub fn into_cached_value(self) -> CachedValue<U> {
        CachedValue::new(self.data, self.expired)
    }
}

#[derive(Default)]
pub struct JsonSerializer<Raw = Vec<u8>> {
    _raw: PhantomData<Raw>,
}

impl Serializer for JsonSerializer<Vec<u8>> {
    type Raw = Vec<u8>;

    fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
    where
        T: DeserializeOwned,
    {
        let deserialized: SerializableCachedValue<T> = serde_json::from_slice(&data)
            .map_err(|err| SerializerError::Deserialize(Box::new(err)))?;
        let cached_value = deserialized.into_cached_value();
        Ok(CachedValue::new(cached_value.data, cached_value.expired))
    }

    fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
    where
        T: Serialize,
    {
        let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
            data: &value.data,
            expired: value.expired,
        };
        serde_json::to_vec(&serializable_value)
            .map_err(|err| SerializerError::Serialize(Box::new(err)))
    }
}

impl Serializer for JsonSerializer<String> {
    type Raw = String;

    fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
    where
        T: DeserializeOwned,
    {
        let deserialized: SerializableCachedValue<T> = serde_json::from_str(&data)
            .map_err(|err| SerializerError::Deserialize(Box::new(err)))?;
        let cached_value = deserialized.into_cached_value();
        Ok(CachedValue::new(cached_value.data, cached_value.expired))
    }

    fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
    where
        T: Serialize,
    {
        let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
            data: &value.data,
            expired: value.expired,
        };
        serde_json::to_string(&serializable_value)
            .map_err(|err| SerializerError::Serialize(Box::new(err)))
    }
}

#[derive(Default)]
pub struct BinSerializer<Raw = Vec<u8>> {
    _raw: PhantomData<Raw>,
}

impl Serializer for BinSerializer<Vec<u8>> {
    type Raw = Vec<u8>;

    fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
    where
        T: DeserializeOwned,
    {
        let deserialized: SerializableCachedValue<T> = bincode::deserialize(&data)
            .map_err(|err| SerializerError::Deserialize(Box::new(err)))?;
        let cached_value = deserialized.into_cached_value();
        Ok(CachedValue::new(cached_value.data, cached_value.expired))
    }

    fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
    where
        T: Serialize,
    {
        let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
            data: &value.data,
            expired: value.expired,
        };
        bincode::serialize(&serializable_value)
            .map_err(|err| SerializerError::Serialize(Box::new(err)))
    }
}

#[cfg(test)]
mod test {
    use async_trait::async_trait;
    use hitbox_core::{CachePolicy, CacheableResponse, PredicateResult};

    use super::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Test {
        a: i32,
        b: String,
    }

    #[async_trait]
    impl CacheableResponse for Test {
        type Cached = Self;
        type Subject = Self;

        async fn cache_policy<P>(self, predicates: P) -> hitbox_core::ResponseCachePolicy<Self>
        where
            P: hitbox_core::Predicate<Subject = Self::Subject> + Send + Sync,
        {
            match predicates.check(self).await {
                PredicateResult::Cacheable(cacheable) => match cacheable.into_cached().await {
                    CachePolicy::Cacheable(res) => {
                        CachePolicy::Cacheable(CachedValue::new(res, Utc::now()))
                    }
                    CachePolicy::NonCacheable(res) => CachePolicy::NonCacheable(res),
                },
                PredicateResult::NonCacheable(res) => CachePolicy::NonCacheable(res),
            }
        }

        async fn into_cached(self) -> CachePolicy<Self::Cached, Self> {
            CachePolicy::Cacheable(self)
        }
        async fn from_cached(cached: Self::Cached) -> Self {
            cached
        }
    }

    impl Test {
        pub fn new() -> Self {
            Self {
                a: 42,
                b: "nope".to_owned(),
            }
        }
    }

    #[test]
    fn test_json_bytes_serializer() {
        let value = CachedValue::new(Test::new(), Utc::now());
        let raw = <JsonSerializer>::serialize(&value).unwrap();
        assert_eq!(value, <JsonSerializer>::deserialize(raw).unwrap());
    }

    #[test]
    fn test_json_string_serializer() {
        let value = CachedValue::new(Test::new(), Utc::now());
        let raw = JsonSerializer::<String>::serialize(&value).unwrap();
        assert_eq!(raw.len(), 71);
        assert_eq!(value, JsonSerializer::<String>::deserialize(raw).unwrap());
    }

    #[test]
    fn test_bincode_serializer() {
        let value = CachedValue::new(Test::new(), Utc::now());
        let raw = <BinSerializer>::serialize(&value).unwrap();
        assert_eq!(raw.len(), 54);
        assert_eq!(value, BinSerializer::<Vec<u8>>::deserialize(raw).unwrap());
    }
}
