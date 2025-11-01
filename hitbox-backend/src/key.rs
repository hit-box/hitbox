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
    /// Debug format: human-readable key-value pairs with quoted values
    /// Format: version: 0\nprefix: ""\nkey: "value"\nkey2: null
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
                // Format: key: "value"\nkey2: null
                // Optional: version: N (if not 0) and prefix: "..." (if not empty)
                let mut output = String::new();

                // Only include version if non-default (not 0)
                if key.version() != 0 {
                    output.push_str(&format!("version: {}\n", key.version()));
                }

                // Only include prefix if non-default (not empty)
                if !key.prefix().is_empty() {
                    let prefix_json = serde_json::to_string(key.prefix())
                        .map_err(|e| SerializerError::Serialize(Box::new(e)))?;
                    output.push_str(&format!("prefix: {}\n", prefix_json));
                }

                // Each part: key: "value" or key: null
                for part in key.parts() {
                    let value_str = match part.value() {
                        None => "null".to_string(),
                        Some(v) => serde_json::to_string(v)
                            .map_err(|e| SerializerError::Serialize(Box::new(e)))?,
                    };
                    output.push_str(&format!("{}: {}\n", part.key(), value_str));
                }

                Ok(output.into_bytes())
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
                // Parse format: key: "value"\nkey2: null
                // Optional: version: N and prefix: "..."
                let s = String::from_utf8_lossy(data).trim().to_string();
                let mut lines = s.lines().peekable();

                // Defaults
                let mut version = 0u32;
                let mut prefix = String::new();

                // Try to parse version line (optional)
                if let Some(first_line) = lines.peek() {
                    if let Some(version_str) = first_line.strip_prefix("version: ") {
                        version = version_str.parse::<u32>()
                            .map_err(|e| SerializerError::Deserialize(Box::new(e)))?;
                        lines.next(); // Consume the version line
                    }
                }

                // Try to parse prefix line (optional)
                if let Some(second_line) = lines.peek() {
                    if let Some(prefix_json) = second_line.strip_prefix("prefix: ") {
                        prefix = serde_json::from_str(prefix_json)
                            .map_err(|e| SerializerError::Deserialize(Box::new(e)))?;
                        lines.next(); // Consume the prefix line
                    }
                }

                // Parse key-value pairs
                let mut parts = Vec::new();
                for line in lines {
                    if line.trim().is_empty() {
                        continue;
                    }

                    // Split on first ': ' to get key and value
                    let (key, value_str) = line
                        .split_once(": ")
                        .ok_or_else(|| SerializerError::Deserialize(Box::new(
                            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid key-value format")
                        )))?;

                    // Parse value - either "null" or JSON-encoded string
                    let value = if value_str == "null" {
                        None
                    } else {
                        let v: String = serde_json::from_str(value_str)
                            .map_err(|e| SerializerError::Deserialize(Box::new(e)))?;
                        Some(v)
                    };

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
