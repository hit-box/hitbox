use thiserror::Error;

/// Configuration error wrapper for serde-saphyr errors
#[derive(Debug, Error)]
#[error("{0}")]
pub struct ConfigError(#[source] serde_saphyr::Error);

impl From<serde_saphyr::Error> for ConfigError {
    fn from(error: serde_saphyr::Error) -> Self {
        Self(error)
    }
}

/// Parse YAML configuration from a string
pub fn parse_config<T>(yaml: &str) -> Result<T, ConfigError>
where
    T: serde::de::DeserializeOwned,
{
    serde_saphyr::from_str(yaml).map_err(ConfigError::from)
}
