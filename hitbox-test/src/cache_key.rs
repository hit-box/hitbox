use hitbox_backend::format::FormatError;
use hitbox_core::CacheKey;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Helper struct for YAML serialization supporting duplicate keys
#[derive(Debug)]
struct FlatCacheKey {
    version: u32,
    prefix: String,
    parts: Vec<(String, Option<String>)>,
}

impl Serialize for FlatCacheKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        if self.version != 0 {
            map.serialize_entry("__v", &self.version)?;
        }
        if !self.prefix.is_empty() {
            map.serialize_entry("__p", &self.prefix)?;
        }
        for (key, value) in &self.parts {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for FlatCacheKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(FlatCacheKeyVisitor)
    }
}

struct FlatCacheKeyVisitor;

impl<'de> Visitor<'de> for FlatCacheKeyVisitor {
    type Value = FlatCacheKey;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map with cache key parts")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut version = 0u32;
        let mut prefix = String::new();
        let mut parts = Vec::new();

        while let Some(key) = access.next_key::<String>()? {
            match key.as_str() {
                // Use "__v" and "__p" as reserved field names
                // to avoid collision with KeyPart names like "version"
                "__v" => version = access.next_value()?,
                "__p" => prefix = access.next_value()?,
                _ => {
                    let value: Option<String> = access.next_value()?;
                    parts.push((key, value));
                }
            }
        }

        Ok(FlatCacheKey {
            version,
            prefix,
            parts,
        })
    }
}

impl From<&CacheKey> for FlatCacheKey {
    fn from(key: &CacheKey) -> Self {
        let parts = key
            .parts()
            .map(|part| (part.key().to_string(), part.value().map(str::to_string)))
            .collect();

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
pub fn serialize_debug(key: &CacheKey) -> Result<Vec<u8>, FormatError> {
    let flat: FlatCacheKey = key.into();
    let yaml_string =
        serde_saphyr::to_string(&flat).map_err(|e| FormatError::Serialize(Box::new(e)))?;
    Ok(yaml_string.into_bytes())
}

/// Deserialize cache key from debug YAML format
pub fn deserialize_debug(data: &[u8]) -> Result<CacheKey, FormatError> {
    let flat: FlatCacheKey =
        serde_saphyr::from_slice(data).map_err(|e| FormatError::Deserialize(Box::new(e)))?;
    Ok(flat.into())
}
