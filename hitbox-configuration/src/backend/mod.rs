use crate::error::ConfigError;
use hitbox_backend::serializer::{BincodeFormat, Format, JsonFormat};
use hitbox_backend::{Backend as BackendTrait, CacheKeyFormat};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Backend {
    Moka(BackendConfig<Moka>),
    FeOxDb(BackendConfig<FeOxDb>),
    Redis(BackendConfig<Redis>),
}

impl Backend {
    pub fn into_backend(self) -> Result<Arc<dyn BackendTrait + Send + 'static>, ConfigError> {
        match self {
            #[cfg(feature = "moka")]
            Backend::Moka(config) => {
                use hitbox_moka::MokaBackend;

                let key_format = config.key.format.to_cache_key_format();
                let serializer = config.value.format.to_serializer();
                let compressor = config.value.compression.to_compressor()?;

                let backend = MokaBackend::builder(config.backend.max_capacity)
                    .key_format(key_format)
                    .value_format(serializer)
                    .compressor(compressor)
                    .build();

                Ok(Arc::new(backend))
            }
            #[cfg(not(feature = "moka"))]
            Backend::Moka(_) => Err(ConfigError::BackendNotAvailable("Moka".to_string())),
            #[cfg(feature = "feoxdb")]
            Backend::FeOxDb(config) => {
                use hitbox_feoxdb::FeOxDbBackend;

                let key_format = config.key.format.to_cache_key_format();
                let serializer = config.value.format.to_serializer();
                let compressor = config.value.compression.to_compressor()?;

                let mut builder = FeOxDbBackend::builder()
                    .key_format(key_format)
                    .value_format(serializer)
                    .compressor(compressor);

                if let Some(path) = config.backend.path {
                    builder = builder.path(path);
                }

                let backend = builder
                    .build()
                    .map_err(|e| ConfigError::BackendNotAvailable(format!("FeOxDb: {}", e)))?;

                Ok(Arc::new(backend))
            }
            #[cfg(not(feature = "feoxdb"))]
            Backend::FeOxDb(_) => Err(ConfigError::BackendNotAvailable("FeOxDb".to_string())),
            #[cfg(feature = "redis")]
            Backend::Redis(config) => {
                use hitbox_redis::RedisBackend;

                let key_format = config.key.format.to_cache_key_format();
                let serializer = config.value.format.to_serializer();
                let compressor = config.value.compression.to_compressor()?;

                let backend = RedisBackend::builder()
                    .server(config.backend.connection_string)
                    .key_format(key_format)
                    .value_format(serializer)
                    .compressor(compressor)
                    .build()
                    .map_err(|e| ConfigError::BackendNotAvailable(format!("Redis: {}", e)))?;

                Ok(Arc::new(backend))
            }
            #[cfg(not(feature = "redis"))]
            Backend::Redis(_) => Err(ConfigError::BackendNotAvailable("Redis".to_string())),
        }
    }
}

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
}

impl ValueSerialization {
    pub fn to_serializer(&self) -> Arc<dyn Format> {
        match self {
            ValueSerialization::Json => Arc::new(JsonFormat),
            ValueSerialization::Bincode => Arc::new(BincodeFormat),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(tag = "type")]
pub enum Compression {
    #[default]
    Disabled,
    Gzip {
        #[serde(default = "default_gzip_level")]
        level: u32,
    },
    Zstd {
        #[serde(default = "default_zstd_level")]
        level: i32,
    },
}

fn default_gzip_level() -> u32 {
    6
}

fn default_zstd_level() -> i32 {
    3
}

impl Compression {
    /// Convert configuration compression format to backend compressor
    pub fn to_compressor(
        &self,
    ) -> Result<std::sync::Arc<dyn hitbox_backend::Compressor>, ConfigError> {
        use hitbox_backend::PassthroughCompressor;
        use std::sync::Arc;

        match self {
            Compression::Disabled => Ok(Arc::new(PassthroughCompressor)),
            #[cfg(feature = "gzip")]
            Compression::Gzip { level } => {
                use hitbox_backend::GzipCompressor;
                Ok(Arc::new(GzipCompressor::with_level(*level)))
            }
            #[cfg(not(feature = "gzip"))]
            Compression::Gzip { .. } => Err(ConfigError::BackendNotAvailable(
                "Gzip compression requested but 'gzip' feature is not enabled".to_string(),
            )),
            #[cfg(feature = "zstd")]
            Compression::Zstd { level } => {
                use hitbox_backend::ZstdCompressor;
                Ok(Arc::new(ZstdCompressor::with_level(*level)))
            }
            #[cfg(not(feature = "zstd"))]
            Compression::Zstd { .. } => Err(ConfigError::BackendNotAvailable(
                "Zstd compression requested but 'zstd' feature is not enabled".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Moka {
    pub max_capacity: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FeOxDb {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Redis {
    pub connection_string: String,
}
