use std::marker::PhantomData;

use hitbox_core::CacheKey;

use crate::serializer::FormatError;

pub trait KeySerializer {
    type Output;

    fn serialize(key: &CacheKey) -> Result<Self::Output, FormatError>;
}

pub struct UrlEncodedKeySerializer<Output = String> {
    _output: PhantomData<Output>,
}

impl KeySerializer for UrlEncodedKeySerializer<String> {
    type Output = String;

    fn serialize(key: &CacheKey) -> Result<Self::Output, FormatError> {
        let parts = key
            .parts()
            .map(|part| (part.key(), part.value()))
            .collect::<Vec<_>>();
        serde_urlencoded::to_string(parts).map_err(|err| FormatError::Serialize(Box::new(err)))
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheKeyFormat {
    /// Bitcode format (most compact binary)
    #[default]
    Bitcode,
    /// URL-encoded format
    UrlEncoded,
}

impl CacheKeyFormat {
    pub fn serialize(&self, key: &CacheKey) -> Result<Vec<u8>, FormatError> {
        match self {
            CacheKeyFormat::Bitcode => Ok(bitcode::encode(key)),
            CacheKeyFormat::UrlEncoded => {
                let parts = key
                    .parts()
                    .map(|part| (part.key(), part.value()))
                    .collect::<Vec<_>>();
                serde_urlencoded::to_string(parts)
                    .map(|s| s.into_bytes())
                    .map_err(|err| FormatError::Serialize(Box::new(err)))
            }
        }
    }

    pub fn deserialize(&self, data: &[u8]) -> Result<CacheKey, FormatError> {
        match self {
            CacheKeyFormat::Bitcode => {
                bitcode::decode(data).map_err(|err| FormatError::Deserialize(Box::new(err)))
            }
            CacheKeyFormat::UrlEncoded => {
                // URL-encoded is one-way for cache keys (used for storage key, not round-trip)
                Err(FormatError::Deserialize(Box::new(std::io::Error::other(
                    "UrlEncoded format deserialization not implemented",
                ))))
            }
        }
    }
}
