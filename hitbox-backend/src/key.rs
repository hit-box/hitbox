//! Cache key serialization formats.
//!
//! This module provides different strategies for serializing [`CacheKey`] to bytes
//! for storage in backends.
//!
//! # Available Formats
//!
//! | Format | Size | Reversible | Use Case |
//! |--------|------|------------|----------|
//! | [`Bitcode`](CacheKeyFormat::Bitcode) | Compact | Yes | Default, most backends |
//! | [`UrlEncoded`](CacheKeyFormat::UrlEncoded) | Larger | Yes | Debugging, human-readable keys |
//!
//! Both formats start every key with [`WIRE_VERSION`], so entries written by a hitbox version with
//! a different storage layout are never read: they miss and stay in the store until they expire.

use std::{iter::once, str::from_utf8};

use hitbox_core::{CacheKey, KeyPart};
use thiserror::Error;

use crate::format::FormatError;

/// The version of everything hitbox stores: key and value encodings, tag timestamps, the
/// composition envelope and the layouts of protocol crates such as hitbox-http.
///
/// Every encoded key starts with it, so two hitbox versions that store data differently never
/// read each other's entries. It is raised only in breaking releases, when a stored layout
/// changes; golden-bytes tests under `tests/golden/v{WIRE_VERSION}/` fail when one does.
pub const WIRE_VERSION: u8 = 1;

const WIRE_KEY: &str = "_wire";
const PREFIX_KEY: &str = "_prefix";
const VERSION_KEY: &str = "_version";

/// A key written by a hitbox version with another storage layout.
#[derive(Debug, Error)]
#[error("cache key has wire version {0}, expected {WIRE_VERSION}")]
pub struct WireVersionMismatch(String);

/// Cache key serialization format.
///
/// Determines how [`CacheKey`] is converted to bytes for storage.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheKeyFormat {
    /// Compact binary format using bitcode, after one [`WIRE_VERSION`] byte.
    ///
    /// Produces the smallest keys. This is the default and recommended format.
    #[default]
    Bitcode,

    /// URL-encoded query string format.
    ///
    /// Produces human-readable keys like `_wire=1&_prefix=api&_version=1&user=123`.
    /// Useful for debugging and human-readable storage keys.
    UrlEncoded,
}

impl CacheKeyFormat {
    /// Serialize a cache key to bytes.
    pub fn serialize(&self, key: &CacheKey) -> Result<Vec<u8>, FormatError> {
        match self {
            CacheKeyFormat::Bitcode => {
                let encoded = bitcode::encode(key);
                let mut bytes = Vec::with_capacity(1 + encoded.len());
                bytes.push(WIRE_VERSION);
                bytes.extend_from_slice(&encoded);
                Ok(bytes)
            }
            CacheKeyFormat::UrlEncoded => {
                let pairs = once((WIRE_KEY, WIRE_VERSION.to_string()))
                    .chain(once((PREFIX_KEY, key.prefix().to_string())))
                    .chain(once((VERSION_KEY, key.version().to_string())))
                    .chain(
                        key.parts()
                            .map(|p| (p.key(), p.value().unwrap_or_default().to_string())),
                    )
                    .collect::<Vec<_>>();

                serde_urlencoded::to_string(pairs)
                    .map(String::into_bytes)
                    .map_err(|err| FormatError::Serialize(Box::new(err)))
            }
        }
    }

    /// Deserialize bytes back to a cache key.
    ///
    /// Fails if the key was written with another [`WIRE_VERSION`].
    pub fn deserialize(&self, data: &[u8]) -> Result<CacheKey, FormatError> {
        match self {
            CacheKeyFormat::Bitcode => match data.split_first() {
                Some((&WIRE_VERSION, rest)) => {
                    bitcode::decode(rest).map_err(|err| FormatError::Deserialize(Box::new(err)))
                }
                other => {
                    let found = other.map_or_else(|| "none".to_owned(), |(v, _)| v.to_string());
                    Err(FormatError::Deserialize(Box::new(WireVersionMismatch(
                        found,
                    ))))
                }
            },
            CacheKeyFormat::UrlEncoded => {
                let input =
                    from_utf8(data).map_err(|err| FormatError::Deserialize(Box::new(err)))?;

                let pairs: Vec<(String, String)> = serde_urlencoded::from_str(input)
                    .map_err(|err| FormatError::Deserialize(Box::new(err)))?;

                let (mut wire, mut prefix, mut version, mut parts) =
                    (None, String::new(), 0u32, Vec::new());

                for (key, value) in pairs {
                    match key.as_str() {
                        WIRE_KEY => wire = Some(value),
                        PREFIX_KEY => prefix = value,
                        VERSION_KEY => {
                            version = value
                                .parse()
                                .map_err(|err| FormatError::Deserialize(Box::new(err)))?;
                        }
                        _ => {
                            let v = if value.is_empty() { None } else { Some(value) };
                            parts.push(KeyPart::new(key, v));
                        }
                    }
                }

                if wire.as_deref() != Some(WIRE_VERSION.to_string().as_str()) {
                    let found = wire.unwrap_or_else(|| "none".to_owned());
                    return Err(FormatError::Deserialize(Box::new(WireVersionMismatch(
                        found,
                    ))));
                }

                Ok(CacheKey::new(prefix, version, parts))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::assert_golden;

    fn key() -> CacheKey {
        CacheKey::new(
            "api",
            3,
            vec![
                KeyPart::new("user", Some("42")),
                KeyPart::new("flag", None::<&str>),
            ],
        )
    }

    #[test]
    fn bitcode_layout_is_stable() {
        let bytes = CacheKeyFormat::Bitcode.serialize(&key()).unwrap();
        assert_eq!(bytes[0], WIRE_VERSION);
        assert_golden("key_bitcode.bin", &bytes);
    }

    #[test]
    fn url_encoded_layout_is_stable() {
        let bytes = CacheKeyFormat::UrlEncoded.serialize(&key()).unwrap();
        assert_eq!(
            String::from_utf8(bytes.clone()).unwrap(),
            "_wire=1&_prefix=api&_version=3&user=42&flag="
        );
        assert_golden("key_url_encoded.txt", &bytes);
    }

    #[test]
    fn keys_of_another_wire_version_are_rejected() {
        let mut bitcode = CacheKeyFormat::Bitcode.serialize(&key()).unwrap();
        bitcode[0] = WIRE_VERSION + 1;
        assert!(CacheKeyFormat::Bitcode.deserialize(&bitcode).is_err());
        assert!(CacheKeyFormat::Bitcode.deserialize(&[]).is_err());

        let url = "_wire=2&_prefix=api&_version=3&user=42";
        let err = CacheKeyFormat::UrlEncoded
            .deserialize(url.as_bytes())
            .err()
            .unwrap();
        assert!(err.to_string().contains("wire version 2"), "{err}");
        let unversioned = "_prefix=api&_version=3&user=42";
        assert!(
            CacheKeyFormat::UrlEncoded
                .deserialize(unversioned.as_bytes())
                .is_err()
        );
    }
}
