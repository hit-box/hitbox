use std::marker::PhantomData;
use std::ops::Deref;

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
struct SerializableCachedValue<U> {
    data: U,
    expired: DateTime<Utc>,
}

impl<U> SerializableCachedValue<U> {
    pub fn into_cached_value(self) -> CachedValue<U> {
        CachedValue::new(self.data, self.expired)
    }
}

// TODO: remove clone
#[derive(Default, Clone)]
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

impl Serializer for bytes::Bytes {
    type Raw = bytes::Bytes;

    fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
    where
        T: DeserializeOwned,
    {
        let deserialized = bincode::deserialize::<SerializableCachedValue<T>>(data.deref())
            .map_err(|err| SerializerError::Deserialize(Box::new(err)))?;
        let cached_value = deserialized.into_cached_value();
        Ok(CachedValue::new(cached_value.data, cached_value.expired))
    }

    fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
    where
        T: Serialize,
    {
        let serializable_value = SerializableCachedValue {
            data: &value.data,
            expired: value.expired,
        };
        bincode::serialize(&serializable_value)
            .map(Into::into)
            .map_err(|err| SerializerError::Serialize(Box::new(err)))
    }
}

#[cfg(test)]
mod test {
    use async_trait::async_trait;
    use hitbox_core::CacheableResponse;

    use super::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Test {
        a: i32,
        b: String,
    }

    #[async_trait]
    impl CacheableResponse for Test {
        type Cached = Self;

        async fn into_cached(self) -> Self::Cached {
            self
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
