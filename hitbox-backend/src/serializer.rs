use std::marker::PhantomData;

use crate::{CachePolicy, CacheableResponse, CachedValue};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub trait Serializer {
    type Raw;

    fn deserialize<T, U>(data: Self::Raw) -> Result<CachedValue<T>, ()>
    where
        U: DeserializeOwned,
        T: CacheableResponse<Cached = U>;
    fn serialize<T, U>(value: CachedValue<T>) -> Result<Option<Self::Raw>, ()>
    where
        T: CacheableResponse<Cached = U>,
        U: Serialize;
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
}

impl<T, U> From<CachedValue<T>> for Option<SerializableCachedValue<U>>
where
    T: CacheableResponse<Cached = U>,
{
    fn from(value: CachedValue<T>) -> Self {
        match value.data.into_cache_policy() {
            CachePolicy::Cacheable(data) => Some(SerializableCachedValue::new(data, value.expired)),
            CachePolicy::NonCacheable(_) => None,
        }
    }
}

impl<T, U> From<SerializableCachedValue<U>> for CachedValue<T>
where
    T: CacheableResponse<Cached = U>,
{
    fn from(value: SerializableCachedValue<U>) -> Self {
        CachedValue::new(T::from_cached(value.data), value.expired)
    }
}

#[derive(Default)]
pub struct JsonSerializer<Raw=Vec<u8>> {
    _raw: PhantomData<Raw>,
}

impl Serializer for JsonSerializer<Vec<u8>> {
    type Raw = Vec<u8>;

    fn deserialize<T, U>(data: Self::Raw) -> Result<CachedValue<T>, ()> 
    where
        U: DeserializeOwned,
        T: CacheableResponse<Cached = U>,
    {
        let deserialized: SerializableCachedValue<U> = serde_json::from_slice(&data).map_err(|_| ())?;
        Ok(CachedValue::from(deserialized))
    }

    fn serialize<T, U>(value: CachedValue<T>) -> Result<Option<Self::Raw>, ()>
    where
        T: CacheableResponse<Cached = U>,
        U: Serialize,
    {
        let serializable_value: Option<SerializableCachedValue<U>> = value.into();
        match serializable_value {
            Some(value) => Ok(Some(serde_json::to_vec(&value).map_err(|_| ())?)),
            None => Ok(None),
        }
    }
}

impl Serializer for JsonSerializer<String> {
    type Raw = String;

    fn deserialize<T, U>(data: Self::Raw) -> Result<CachedValue<T>, ()> 
    where
        U: DeserializeOwned,
        T: CacheableResponse<Cached = U>,
    {
        let deserialized: SerializableCachedValue<U> = serde_json::from_str(&data).map_err(|_| ())?;
        Ok(CachedValue::from(deserialized))
    }

    fn serialize<T, U>(value: CachedValue<T>) -> Result<Option<Self::Raw>, ()>
    where
        T: CacheableResponse<Cached = U>,
        U: Serialize,
    {
        let serializable_value: Option<SerializableCachedValue<U>> = value.into();
        match serializable_value {
            Some(value) => Ok(Some(serde_json::to_string(&value).map_err(|_| ())?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::CacheableResponse;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Test {
        a: i32,
        b: String,
    }

    impl CacheableResponse for Test {
        type Cached = Self;

        fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
            CachePolicy::Cacheable(self)
        }

        fn from_cached(cached: Self::Cached) -> Self {
            cached
        }

        fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
            CachePolicy::Cacheable(self)
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
        let raw = <JsonSerializer>::serialize(value.clone()).unwrap().unwrap();
        assert_eq!(value, <JsonSerializer>::deserialize(raw).unwrap());
    }

    #[test]
    fn test_json_string_serializer() {
        let value = CachedValue::new(Test::new(), Utc::now());
        let raw = JsonSerializer::<String>::serialize(value.clone()).unwrap().unwrap();
        dbg!(&raw);
        assert_eq!(value, JsonSerializer::<String>::deserialize(raw).unwrap());
    }
}
