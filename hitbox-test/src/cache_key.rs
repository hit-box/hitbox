use hitbox_backend::serializer::SerializerError;
use hitbox_core::CacheKey;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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

/// Serialize cache key in debug YAML format
pub fn serialize_debug(key: &CacheKey) -> Result<Vec<u8>, SerializerError> {
    let flat: FlatCacheKey = key.into();
    let yaml_string =
        serde_yaml::to_string(&flat).map_err(|e| SerializerError::Serialize(Box::new(e)))?;
    Ok(yaml_string.into_bytes())
}

/// Deserialize cache key from debug YAML format
pub fn deserialize_debug(data: &[u8]) -> Result<CacheKey, SerializerError> {
    let flat: FlatCacheKey =
        serde_yaml::from_slice(data).map_err(|e| SerializerError::Deserialize(Box::new(e)))?;
    Ok(flat.into())
}
