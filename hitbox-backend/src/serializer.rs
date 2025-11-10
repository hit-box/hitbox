use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormatError {
    #[error(transparent)]
    Serialize(Box<dyn std::error::Error + Send>),

    #[error(transparent)]
    Deserialize(Box<dyn std::error::Error + Send>),
}

pub type Raw = Vec<u8>;

/// Object-safe format trait (uses erased-serde for type erasure)
/// This trait can be used with `Arc<dyn Format>` for dynamic dispatch
pub trait Format: std::fmt::Debug + Send + Sync {
    fn erased_serialize(&self, value: &dyn erased_serde::Serialize) -> Result<Raw, FormatError>;

    /// Provides access to a deserializer via a callback to avoid lifetime issues
    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut dyn erased_serde::Deserializer) -> Result<(), erased_serde::Error>,
    ) -> Result<(), FormatError>;
}

/// Extension trait providing generic serialize/deserialize methods
/// This is automatically implemented for all Format types
pub trait FormatExt: Format {
    fn serialize<T>(&self, value: &T) -> Result<Raw, FormatError>
    where
        T: Serialize,
    {
        self.erased_serialize(value as _)
    }

    fn deserialize<T>(&self, data: &Raw) -> Result<T, FormatError>
    where
        T: DeserializeOwned,
    {
        let mut result: Option<T> = None;
        self.with_deserializer(data, &mut |deserializer| {
            let value: T = erased_serde::deserialize(deserializer)?;
            result = Some(value);
            Ok(())
        })
        .map_err(|error| FormatError::Deserialize(Box::new(error)))?;

        result.ok_or_else(|| {
            FormatError::Deserialize(Box::new(std::io::Error::other(
                "deserialization produced no result",
            )))
        })
    }
}

// Blanket implementation: all Formats automatically get generic methods
impl<T: Format + ?Sized> FormatExt for T {}

// Blanket implementation for Arc<dyn Format>
impl Format for std::sync::Arc<dyn Format> {
    fn erased_serialize(&self, value: &dyn erased_serde::Serialize) -> Result<Raw, FormatError> {
        (**self).erased_serialize(value)
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut dyn erased_serde::Deserializer) -> Result<(), erased_serde::Error>,
    ) -> Result<(), FormatError> {
        (**self).with_deserializer(data, f)
    }
}

/// JSON format (default)
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonFormat;

impl Format for JsonFormat {
    fn erased_serialize(&self, value: &dyn erased_serde::Serialize) -> Result<Raw, FormatError> {
        let mut buf = Vec::new();
        let mut ser = serde_json::Serializer::new(&mut buf);
        value
            .erased_serialize(&mut <dyn erased_serde::Serializer>::erase(&mut ser))
            .map_err(|error| FormatError::Serialize(Box::new(error)))?;
        Ok(buf)
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut dyn erased_serde::Deserializer) -> Result<(), erased_serde::Error>,
    ) -> Result<(), FormatError> {
        let mut deser = serde_json::Deserializer::from_slice(data);
        let mut erased = <dyn erased_serde::Deserializer>::erase(&mut deser);
        f(&mut erased).map_err(|error| FormatError::Deserialize(Box::new(error)))
    }
}

/// Bincode format
#[derive(Debug, Clone, Copy)]
pub struct BincodeFormat;

impl Format for BincodeFormat {
    fn erased_serialize(&self, value: &dyn erased_serde::Serialize) -> Result<Raw, FormatError> {
        // Wrapper that implements serde::Serialize
        struct SerdeWrapper<'a>(&'a dyn erased_serde::Serialize);

        impl<'a> serde::Serialize for SerdeWrapper<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::Error;
                erased_serde::serialize(self.0, serializer).map_err(S::Error::custom)
            }
        }

        bincode::serde::encode_to_vec(SerdeWrapper(value), bincode::config::standard())
            .map_err(|error| FormatError::Serialize(Box::new(error)))
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut dyn erased_serde::Deserializer) -> Result<(), erased_serde::Error>,
    ) -> Result<(), FormatError> {
        use bincode::de::read::SliceReader;
        use bincode::serde::OwnedSerdeDecoder;

        let reader = SliceReader::new(data);
        let mut decoder = OwnedSerdeDecoder::from_reader(reader, bincode::config::standard());
        let deser = decoder.as_deserializer();

        // Erase the bincode deserializer
        let mut erased = <dyn erased_serde::Deserializer>::erase(deser);
        f(&mut erased).map_err(|error| FormatError::Deserialize(Box::new(error)))
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
//     fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, FormatError>
//     where
//         T: DeserializeOwned;
//
//     fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, FormatError>
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
//     fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, FormatError>
//     where
//         T: DeserializeOwned,
//     {
//         let deserialized: SerializableCachedValue<T> = serde_json::from_slice(&data)
//             .map_err(|err| FormatError::Deserialize(Box::new(err)))?;
//         let cached_value = deserialized.into_cached_value();
//         Ok(CachedValue::new(cached_value.data, cached_value.expired))
//     }
//
//     fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, FormatError>
//     where
//         T: Serialize,
//     {
//         let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
//             data: &value.data,
//             expired: value.expired,
//         };
//         serde_json::to_vec(&serializable_value)
//             .map_err(|err| FormatError::Serialize(Box::new(err)))
//     }
// }
//
// impl Serializer for JsonSerializer<String> {
//     type Raw = String;
//
//     fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, FormatError>
//     where
//         T: DeserializeOwned,
//     {
//         let deserialized: SerializableCachedValue<T> = serde_json::from_str(&data)
//             .map_err(|err| FormatError::Deserialize(Box::new(err)))?;
//         let cached_value = deserialized.into_cached_value();
//         Ok(CachedValue::new(cached_value.data, cached_value.expired))
//     }
//
//     fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, FormatError>
//     where
//         T: Serialize,
//     {
//         let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
//             data: &value.data,
//             expired: value.expired,
//         };
//         serde_json::to_string(&serializable_value)
//             .map_err(|err| FormatError::Serialize(Box::new(err)))
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
//     fn deserialize<T>(data: Self::Raw) -> Result<CachedValue<T>, FormatError>
//     where
//         T: DeserializeOwned,
//     {
//         let deserialized: SerializableCachedValue<T> = bincode::deserialize(&data)
//             .map_err(|err| FormatError::Deserialize(Box::new(err)))?;
//         let cached_value = deserialized.into_cached_value();
//         Ok(CachedValue::new(cached_value.data, cached_value.expired))
//     }
//
//     fn serialize<T>(value: &CachedValue<T>) -> Result<Self::Raw, FormatError>
//     where
//         T: Serialize,
//     {
//         let serializable_value: SerializableCachedValue<&T> = SerializableCachedValue {
//             data: &value.data,
//             expired: value.expired,
//         };
//         bincode::serialize(&serializable_value)
//             .map_err(|err| FormatError::Serialize(Box::new(err)))
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
