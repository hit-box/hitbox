use thiserror::Error;

/// Configuration error types
#[derive(Debug, Error)]
pub enum ConfigError {
    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_saphyr::Error),

    /// Invalid HTTP header name
    #[error("Invalid HTTP header name: {0}")]
    InvalidHeaderName(String, #[source] http::header::InvalidHeaderName),

    /// Invalid HTTP header value
    #[error("Invalid HTTP header value: {0}")]
    InvalidHeaderValue(String, #[source] http::header::InvalidHeaderValue),

    /// Invalid HTTP method
    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(String, #[source] http::method::InvalidMethod),

    /// Invalid HTTP status code
    #[error("Invalid HTTP status code: {0}")]
    InvalidStatusCode(u16),

    /// Invalid regex pattern
    #[error("Invalid regex pattern '{pattern}': {error}")]
    InvalidRegex {
        pattern: String,
        #[source]
        error: regex::Error,
    },

    /// Backend not available (feature not enabled)
    #[error("Backend '{0}' is not available. Enable the corresponding feature flag.")]
    BackendNotAvailable(String),
}

impl From<http::method::InvalidMethod> for ConfigError {
    fn from(e: http::method::InvalidMethod) -> Self {
        ConfigError::InvalidMethod(String::new(), e)
    }
}

/// Parse YAML configuration from a string
pub fn parse_config<T>(yaml: &str) -> Result<T, ConfigError>
where
    T: serde::de::DeserializeOwned,
{
    serde_saphyr::from_str(yaml).map_err(ConfigError::from)
}
