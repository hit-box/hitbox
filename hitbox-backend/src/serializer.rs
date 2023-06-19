use std::marker::PhantomData;

use crate::{
    response::{CachePolicy, CacheableResponse},
    CachedValue,
};
use chrono::{DateTime, Utc};
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
    pub fn new(data: U, expired: DateTime<Utc>) -> Self {
        SerializableCachedValue { data, expired }
    }

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
        Ok(CachedValue::new(
            cached_value.data.into(),
            cached_value.expired,
        ))
    }

    fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
    where
        T: Serialize,
    {
        let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
            data: &value.data,
            expired: value.expired,
        };
        Ok(serde_json::to_vec(&serializable_value)
            .map_err(|err| SerializerError::Serialize(Box::new(err)))?)
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
        Ok(CachedValue::new(
            cached_value.data.into(),
            cached_value.expired,
        ))
    }

    fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
    where
        T: Serialize,
    {
        let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
            data: &value.data,
            expired: value.expired,
        };
        Ok(serde_json::to_string(&serializable_value)
            .map_err(|err| SerializerError::Serialize(Box::new(err)))?)
    }
}

#[cfg(test)]
mod test {
    use std::convert::Infallible;

    use async_trait::async_trait;

    use super::*;
    use crate::{response::CacheableResponse, CacheableResponseWrapper};

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Test {
        a: i32,
        b: String,
    }

    #[async_trait]
    impl CacheableResponseWrapper for Test {
        type Source = Self;
        type Serializable = Self;
        type Error = Infallible;

        fn from_serializable(serializable: Self::Serializable) -> Self {
            serializable
        }

        fn from_source(source: Self::Source) -> Self {
            source
        }

        fn into_source(self) -> Self::Source {
            self
        }

        async fn into_serializable(self) -> Result<Self::Serializable, Self::Error> {
            Ok(self)
        }
    }

    impl CacheableResponse for Test {
        type Cached = Self;

        fn is_cacheable(&self) -> bool {
            true
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
        dbg!(&raw);
        assert_eq!(value, JsonSerializer::<String>::deserialize(raw).unwrap());
    }
}
