use std::marker::PhantomData;

use hitbox_core::CacheKey;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::serializer::SerializerError;

pub trait KeySerializer {
    type Output;

    fn serialize(key: &CacheKey) -> Result<Self::Output, SerializerError>;
}

pub struct UrlEncodedKeySerializer<Output = String> {
    _output: PhantomData<Output>,
}

impl KeySerializer for UrlEncodedKeySerializer<String> {
    type Output = String;

    fn serialize(key: &CacheKey) -> Result<Self::Output, SerializerError> {
        let parts = key
            .parts()
            .map(|part| (part.key(), part.value()))
            .collect::<Vec<_>>();
        serde_urlencoded::to_string(parts).map_err(|err| SerializerError::Serialize(Box::new(err)))
    }
}

/// Helper struct for flattened YAML serialization
#[derive(Serialize, Deserialize)]
struct FlatCacheKey {
    #[serde(skip_serializing_if = "is_default_version")]
    #[serde(default)]
    version: u32,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    prefix: String,

    #[serde(flatten)]
    parts: IndexMap<String, Option<String>>,
}

fn is_default_version(v: &u32) -> bool {
    *v == 0
}

impl From<&CacheKey> for FlatCacheKey {
    fn from(key: &CacheKey) -> Self {
        let mut parts = IndexMap::new();
        for part in key.parts() {
            parts.insert(part.key().clone(), part.value().clone());
        }

        FlatCacheKey {
            version: key.version(),
            prefix: key.prefix().to_string(),
            parts,
        }
    }
}

impl From<FlatCacheKey> for CacheKey {
    fn from(flat: FlatCacheKey) -> Self {
        let parts = flat
            .parts
            .into_iter()
            .map(|(key, value)| hitbox_core::KeyPart::new(key, value))
            .collect();

        CacheKey::new(flat.prefix, flat.version, parts)
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub enum CacheKeyFormat {
    /// Debug format: human-readable YAML with flattened key-value pairs
    /// Format: key: "value"\nkey2: null\nversion: 1 (optional)\nprefix: "api" (optional)
    #[default]
    Debug,
    /// JSON format (structured, debugging)
    Json,
    /// Bincode format (compact binary)
    Bincode,
    /// URL-encoded format
    UrlEncoded,
}

impl CacheKeyFormat {
    pub fn serialize(&self, key: &CacheKey) -> Result<Vec<u8>, SerializerError> {
        match self {
            CacheKeyFormat::Debug => {
                let flat: FlatCacheKey = key.into();
                let yaml_string = serde_yaml::to_string(&flat)
                    .map_err(|e| SerializerError::Serialize(Box::new(e)))?;
                Ok(yaml_string.into_bytes())
            }
            CacheKeyFormat::Json => serde_json::to_vec(key)
                .map_err(|err| SerializerError::Serialize(Box::new(err))),
            CacheKeyFormat::Bincode => bincode::serialize(key)
                .map_err(|err| SerializerError::Serialize(Box::new(err))),
            CacheKeyFormat::UrlEncoded => {
                let parts = key
                    .parts()
                    .map(|part| (part.key(), part.value()))
                    .collect::<Vec<_>>();
                serde_urlencoded::to_string(parts)
                    .map(|s| s.into_bytes())
                    .map_err(|err| SerializerError::Serialize(Box::new(err)))
            }
        }
    }

    pub fn deserialize(&self, data: &[u8]) -> Result<CacheKey, SerializerError> {
        match self {
            CacheKeyFormat::Debug => {
                let flat: FlatCacheKey = serde_yaml::from_slice(data)
                    .map_err(|e| SerializerError::Deserialize(Box::new(e)))?;
                Ok(flat.into())
            }
            CacheKeyFormat::Json => serde_json::from_slice(data)
                .map_err(|err| SerializerError::Deserialize(Box::new(err))),
            CacheKeyFormat::Bincode => bincode::deserialize(data)
                .map_err(|err| SerializerError::Deserialize(Box::new(err))),
            CacheKeyFormat::UrlEncoded => {
                // URL-encoded is one-way for cache keys (used for storage key, not round-trip)
                Err(SerializerError::Deserialize(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "UrlEncoded format deserialization not implemented",
                    ),
                )))
            }
        }
    }
}
