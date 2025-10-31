use std::marker::PhantomData;

use hitbox_core::CacheKey;

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

#[derive(Default, Clone, Copy, Debug)]
pub enum CacheKeyFormat {
    /// Custom string format: "prefix::version::key1:value1::key2:value2"
    #[default]
    String,
    /// JSON format (human-readable, debugging)
    Json,
    /// Bincode format (compact binary)
    Bincode,
    /// URL-encoded format
    UrlEncoded,
}

impl CacheKeyFormat {
    pub fn serialize(&self, key: &CacheKey) -> Result<Vec<u8>, SerializerError> {
        match self {
            CacheKeyFormat::String => {
                let key_parts = key
                    .parts()
                    .map(|part| {
                        format!(
                            "{}:{}",
                            part.key(),
                            part.value().as_deref().unwrap_or("null")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("::");
                let full = format!("{}::{}::{}", key.prefix(), key.version(), key_parts);
                Ok(full.into_bytes())
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
            CacheKeyFormat::String => {
                let s = String::from_utf8_lossy(data);
                let mut sections = s.split("::");

                let prefix = sections.next()
                    .ok_or_else(|| SerializerError::Deserialize(Box::new(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing prefix")
                    )))?
                    .to_string();

                let version = sections.next()
                    .ok_or_else(|| SerializerError::Deserialize(Box::new(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing version")
                    )))?
                    .parse::<u32>()
                    .map_err(|err| SerializerError::Deserialize(Box::new(err)))?;

                let mut parts = Vec::new();
                for section in sections {
                    let mut kv = section.split(':');
                    let key = kv.next()
                        .ok_or_else(|| SerializerError::Deserialize(Box::new(
                            std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing key")
                        )))?;
                    let value = kv.next()
                        .ok_or_else(|| SerializerError::Deserialize(Box::new(
                            std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing value")
                        )))?;

                    let value = if value == "null" { None } else { Some(value.to_string()) };
                    parts.push(hitbox_core::KeyPart::new(key, value));
                }

                Ok(CacheKey::new(prefix, version, parts))
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
