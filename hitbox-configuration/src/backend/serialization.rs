use hitbox_backend::CacheKeyFormat;
use hitbox_backend::format::{BincodeFormat, Format, JsonFormat, PostcardFormat};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::Compression;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BackendConfig<T> {
    pub key: KeyFormat,
    pub value: ValueFormat,
    #[serde(flatten)]
    pub backend: T,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct KeyFormat {
    pub format: KeySerialization,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ValueFormat {
    pub format: ValueSerialization,
    #[serde(default)]
    pub compression: Compression,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum KeySerialization {
    UrlEncoded,
    Bitcode,
}

impl KeySerialization {
    /// Convert configuration key serialization format to backend key format
    pub fn to_cache_key_format(&self) -> CacheKeyFormat {
        match self {
            KeySerialization::UrlEncoded => CacheKeyFormat::UrlEncoded,
            KeySerialization::Bitcode => CacheKeyFormat::Bitcode,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ValueSerialization {
    Json,
    Bincode,
    Postcard,
    #[cfg(feature = "rkyv_format")]
    Rkyv,
}

impl ValueSerialization {
    pub fn to_serializer(&self) -> Arc<dyn Format> {
        match self {
            ValueSerialization::Json => Arc::new(JsonFormat),
            ValueSerialization::Bincode => Arc::new(BincodeFormat),
            ValueSerialization::Postcard => Arc::new(PostcardFormat),
            #[cfg(feature = "rkyv_format")]
            ValueSerialization::Rkyv => {
                use hitbox_backend::format::RkyvFormat;
                Arc::new(RkyvFormat::new())
            }
        }
    }
}
