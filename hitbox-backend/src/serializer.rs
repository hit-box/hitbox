use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerializerError {
    #[error(transparent)]
    Serialize(Box<dyn std::error::Error + Send>),

    #[error(transparent)]
    Deserialize(Box<dyn std::error::Error + Send>),
}

pub type Raw = Vec<u8>;

pub trait Serializer {
    fn serialize<T>(&self, value: &T) -> Result<Raw, SerializerError>
    where
        T: Serialize;

    fn deserialize<T>(&self, value: &Raw) -> Result<T, SerializerError>
    where
        T: DeserializeOwned;
}

pub struct Json;

impl Serializer for Json {
    fn serialize<T>(&self, value: &T) -> Result<Raw, SerializerError>
    where
        T: Serialize,
    {
        serde_json::to_vec(value).map_err(|error| SerializerError::Serialize(Box::new(error)))
    }

    fn deserialize<T>(&self, value: &Raw) -> Result<T, SerializerError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice(value.as_slice())
            .map_err(|error| SerializerError::Deserialize(Box::new(error)))
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    #[default]
    Json,
    Bincode,
}

impl Format {
    pub fn serialize<T>(&self, value: &T) -> Result<Raw, SerializerError>
    where
        T: Serialize,
    {
        match self {
            Format::Json => serde_json::to_vec(value)
                .map_err(|error| SerializerError::Serialize(Box::new(error))),
            Format::Bincode => bincode::serde::encode_to_vec(value, bincode::config::standard())
                .map_err(|error| SerializerError::Serialize(Box::new(error))),
        }
    }

    pub fn deserialize<T>(&self, value: &Raw) -> Result<T, SerializerError>
    where
        T: DeserializeOwned,
    {
        match self {
            Format::Json => serde_json::from_slice(value.as_slice())
                .map_err(|error| SerializerError::Deserialize(Box::new(error))),
            Format::Bincode => {
                bincode::serde::decode_from_slice(value.as_slice(), bincode::config::standard())
                    .map(|(result, _)| result)
                    .map_err(|error| SerializerError::Deserialize(Box::new(error)))
            }
        }
    }
}

#[cfg(test)]
mod test {
    // use serde::Deserialize;
    //
    // use super::*;
    //
    // #[derive(Serialize, Deserialize, Debug)]
    // struct Value {
    //     name: String,
    //     number: u32,
    // }
    //
    // impl Value {
    //     fn new() -> Self {
    //         Value {
    //             name: "name".to_owned(),
    //             number: 42,
    //         }
    //     }
    // }
    //
    // #[test]
    // fn test_processors() {
    //     let serializer = Serializer::builder()
    //         .format(Format::Json)
    //         // .processors(Gzip::new().chain(Gzip::new()))
    //         .processors(Gzip::new())
    //         .build();
    //
    //     let value = Value::new();
    //     let serialized = serializer.serialize(&value);
    //     let deserialized = serializer.deserialize::<Value>(serialized);
    //     dbg!(&deserialized);
    // }
}

// pub trait Serializer {
//     type Raw;
//
//     fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
//     where
//         T: DeserializeOwned;
//
//     fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
//     where
//         T: Serialize;
// }
//
// #[derive(Deserialize, Serialize)]
// struct SerializableCachedValue<U> {
//     data: U,
//     expired: DateTime<Utc>,
// }
//
// impl<U> SerializableCachedValue<U> {
//     pub fn into_cached_value(self) -> CachedValue<U> {
//         CachedValue::new(self.data, self.expired)
//     }
// }
//
// #[derive(Default)]
// pub struct JsonSerializer<Raw = Vec<u8>> {
//     _raw: PhantomData<Raw>,
// }
//
// impl Serializer for JsonSerializer<Vec<u8>> {
//     type Raw = Vec<u8>;
//
//     fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
//     where
//         T: DeserializeOwned,
//     {
//         let deserialized: SerializableCachedValue<T> = serde_json::from_slice(&data)
//             .map_err(|err| SerializerError::Deserialize(Box::new(err)))?;
//         let cached_value = deserialized.into_cached_value();
//         Ok(CachedValue::new(cached_value.data, cached_value.expired))
//     }
//
//     fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
//     where
//         T: Serialize,
//     {
//         let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
//             data: &value.data,
//             expired: value.expired,
//         };
//         serde_json::to_vec(&serializable_value)
//             .map_err(|err| SerializerError::Serialize(Box::new(err)))
//     }
// }
//
// impl Serializer for JsonSerializer<String> {
//     type Raw = String;
//
//     fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
//     where
//         T: DeserializeOwned,
//     {
//         let deserialized: SerializableCachedValue<T> = serde_json::from_str(&data)
//             .map_err(|err| SerializerError::Deserialize(Box::new(err)))?;
//         let cached_value = deserialized.into_cached_value();
//         Ok(CachedValue::new(cached_value.data, cached_value.expired))
//     }
//
//     fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
//     where
//         T: Serialize,
//     {
//         let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
//             data: &value.data,
//             expired: value.expired,
//         };
//         serde_json::to_string(&serializable_value)
//             .map_err(|err| SerializerError::Serialize(Box::new(err)))
//     }
// }
//
// #[derive(Default)]
// pub struct BinSerializer<Raw = Vec<u8>> {
//     _raw: PhantomData<Raw>,
// }
//
// impl Serializer for BinSerializer<Vec<u8>> {
//     type Raw = Vec<u8>;
//
//     fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, SerializerError>
//     where
//         T: DeserializeOwned,
//     {
//         let deserialized: SerializableCachedValue<T> = bincode::deserialize(&data)
//             .map_err(|err| SerializerError::Deserialize(Box::new(err)))?;
//         let cached_value = deserialized.into_cached_value();
//         Ok(CachedValue::new(cached_value.data, cached_value.expired))
//     }
//
//     fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, SerializerError>
//     where
//         T: Serialize,
//     {
//         let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
//             data: &value.data,
//             expired: value.expired,
//         };
//         bincode::serialize(&serializable_value)
//             .map_err(|err| SerializerError::Serialize(Box::new(err)))
//     }
// }
//
// #[cfg(test)]
// mod test {
//     use async_trait::async_trait;
//     use hitbox_core::{CachePolicy, CacheableResponse, PredicateResult};
//
//     use super::*;
//
//     #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
//     struct Test {
//         a: i32,
//         b: String,
//     }
//
//     #[async_trait]
//     impl CacheableResponse for Test {
//         type Cached = Self;
//         type Subject = Self;
//
//         async fn cache_policy<P>(self, predicates: P) -> hitbox_core::ResponseCachePolicy<Self>
//         where
//             P: hitbox_core::Predicate<Subject = Self::Subject> + Send + Sync,
//         {
//             match predicates.check(self).await {
//                 PredicateResult::Cacheable(cacheable) => match cacheable.into_cached().await {
//                     CachePolicy::Cacheable(res) => {
//                         CachePolicy::Cacheable(CachedValue::new(res, Utc::now()))
//                     }
//                     CachePolicy::NonCacheable(res) => CachePolicy::NonCacheable(res),
//                 },
//                 PredicateResult::NonCacheable(res) => CachePolicy::NonCacheable(res),
//             }
//         }
//
//         async fn into_cached(self) -> CachePolicy<Self::Cached, Self> {
//             CachePolicy::Cacheable(self)
//         }
//         async fn from_cached(cached: Self::Cached) -> Self {
//             cached
//         }
//     }
//
//     impl Test {
//         pub fn new() -> Self {
//             Self {
//                 a: 42,
//                 b: "nope".to_owned(),
//             }
//         }
//     }
//
//     #[test]
//     fn test_json_bytes_serializer() {
//         let value = CachedValue::new(Test::new(), Utc::now());
//         let raw = <JsonSerializer>::serialize(&value).unwrap();
//         assert_eq!(value, <JsonSerializer>::deserialize(raw).unwrap());
//     }
//
//     #[test]
//     fn test_json_string_serializer() {
//         let value = CachedValue::new(Test::new(), Utc::now());
//         let raw = JsonSerializer::<String>::serialize(&value).unwrap();
//         assert_eq!(raw.len(), 71);
//         assert_eq!(value, JsonSerializer::<String>::deserialize(raw).unwrap());
//     }
//
//     #[test]
//     fn test_bincode_serializer() {
//         let value = CachedValue::new(Test::new(), Utc::now());
//         let raw = <BinSerializer>::serialize(&value).unwrap();
//         assert_eq!(raw.len(), 54);
//         assert_eq!(value, BinSerializer::<Vec<u8>>::deserialize(raw).unwrap());
//     }
// }
