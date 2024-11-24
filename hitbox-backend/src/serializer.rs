use std::io::{Read, Write};

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SerializerError {
    #[error(transparent)]
    Serialize(Box<dyn std::error::Error + Send>),

    #[error(transparent)]
    Deserialize(Box<dyn std::error::Error + Send>),
}

pub type Raw = Vec<u8>;

#[derive(Debug, Default)]
pub enum Format {
    #[default]
    Json,
    Yaml,
}

impl Format {
    pub fn serialize<T>(&self, value: &T) -> Raw
    where
        T: Serialize,
    {
        match self {
            Format::Json => serde_json::to_vec(value).unwrap(),
            Format::Yaml => {
                let mut writer = Vec::with_capacity(128);
                serde_yaml::to_writer(&mut writer, value).unwrap();
                writer
            }
        }
    }

    pub fn deserialize<T>(&self, value: &Raw) -> T
    where
        T: DeserializeOwned,
    {
        match self {
            Format::Json => serde_json::from_slice(value).unwrap(),
            Format::Yaml => serde_yaml::from_slice(value).unwrap(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Serializer<P = NeuralProcessor> {
    format: Format,
    processors: P,
}

impl Serializer {
    pub fn builder() -> SerializerBuilder<NeuralProcessor> {
        SerializerBuilder {
            format: Format::Json,
            processors: NeuralProcessor::new(),
        }
    }
}

impl<P> Serializer<P>
where
    P: Processor,
{
    pub fn serialize<T>(&self, value: &T) -> Raw
    where
        T: Serialize,
    {
        let value = self.format.serialize(value);
        self.processors.set_preprocess(value)
    }

    pub fn deserialize<T>(&self, value: Raw) -> T
    where
        T: DeserializeOwned,
    {
        let value = self.processors.get_preprocess(value);
        self.format.deserialize(&value)
    }
}

pub struct SerializerBuilder<P> {
    format: Format,
    processors: P,
}

impl<P> SerializerBuilder<P> {
    pub fn format(self, format: Format) -> Self {
        SerializerBuilder { format, ..self }
    }

    pub fn processors<T>(self, processors: T) -> SerializerBuilder<T> {
        SerializerBuilder {
            processors,
            format: self.format,
        }
    }

    pub fn build(self) -> Serializer<P> {
        Serializer {
            format: self.format,
            processors: self.processors,
        }
    }
}

pub trait Processor {
    fn get_preprocess(&self, value: Raw) -> Raw;
    fn set_preprocess(&self, value: Raw) -> Raw;
    // fn chain<P: Processor>(self, inner: P) -> impl Processor;
}

#[derive(Debug, Default)]
pub struct NeuralProcessor {}

impl NeuralProcessor {
    fn new() -> Self {
        NeuralProcessor {}
    }
}

impl Processor for NeuralProcessor {
    #[inline]
    fn get_preprocess(&self, value: Raw) -> Raw {
        value
    }

    #[inline]
    fn set_preprocess(&self, value: Raw) -> Raw {
        value
    }

    // fn chain<P: Processor>(self, inner: P) -> impl Processor {
    // }
}

pub struct Gzip<P> {
    _inner: P,
}

impl Gzip<NeuralProcessor> {
    pub fn new() -> Gzip<NeuralProcessor> {
        Gzip::default()
    }
}

impl Default for Gzip<NeuralProcessor> {
    fn default() -> Self {
        Gzip {
            _inner: NeuralProcessor::new(),
        }
    }
}

impl<P> Processor for Gzip<P> {
    fn get_preprocess(&self, value: Raw) -> Raw {
        let mut decoder = GzDecoder::new(value.as_slice());
        let mut value = Vec::new();
        decoder.read_to_end(&mut value).unwrap();
        value
    }

    fn set_preprocess(&self, value: Raw) -> Raw {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(value.as_slice()).unwrap();
        encoder.finish().unwrap()
    }
}

#[cfg(test)]
mod test {
    use serde::Deserialize;

    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct Value {
        name: String,
        number: u32,
    }

    impl Value {
        fn new() -> Self {
            Value {
                name: "name".to_owned(),
                number: 42,
            }
        }
    }

    #[test]
    fn test_processors() {
        let serializer = Serializer::builder()
            .format(Format::Json)
            // .processors(Gzip::new().chain(Gzip::new()))
            .processors(Gzip::new())
            .build();

        let value = Value::new();
        let serialized = serializer.serialize(&value);
        let deserialized = serializer.deserialize::<Value>(serialized);
        dbg!(&deserialized);
    }
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
