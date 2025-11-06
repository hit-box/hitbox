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
    /// Bitcode format (most compact binary)
    #[default]
    Bitcode,
    /// URL-encoded format
    UrlEncoded,
}

impl CacheKeyFormat {
    pub fn serialize(&self, key: &CacheKey) -> Result<Vec<u8>, SerializerError> {
        match self {
            CacheKeyFormat::Bitcode => Ok(bitcode::encode(key)),
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
            CacheKeyFormat::Bitcode => {
                bitcode::decode(data).map_err(|err| SerializerError::Deserialize(Box::new(err)))
            }
            CacheKeyFormat::UrlEncoded => {
                // URL-encoded is one-way for cache keys (used for storage key, not round-trip)
                Err(SerializerError::Deserialize(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "UrlEncoded format deserialization not implemented",
                ))))
            }
        }
    }
}
