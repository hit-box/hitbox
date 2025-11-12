use thiserror::Error;

/// Configuration error types
#[derive(Debug, Error)]
pub enum ConfigError {
    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_saphyr::Error),

    /// YAML scanner error (from yaml_rust)
    #[cfg(feature = "validation")]
    #[error("YAML parsing error: {0}")]
    YamlScan(String),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Schema validation error
    #[cfg(feature = "validation")]
    #[error("Configuration validation error:\n{0}")]
    Validation(String),

    /// Invalid HTTP header name
    #[error("Invalid HTTP header name: {0}")]
    InvalidHeaderName(String, #[source] http::header::InvalidHeaderName),

    /// Invalid HTTP header value
    #[error("Invalid HTTP header value: {0}")]
    InvalidHeaderValue(String, #[source] http::header::InvalidHeaderValue),

    /// Invalid HTTP method
    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(String, #[source] http::method::InvalidMethod),

    /// Invalid HTTP status code: {0} (must be between 100 and 599)
    #[error("Invalid HTTP status code: {0}")]
    InvalidStatusCode(u16),

    /// Invalid regex pattern
    #[error("Invalid regex pattern '{pattern}': {error}")]
    InvalidRegex {
        pattern: String,
        #[source]
        error: regex::Error,
    },

    /// Invalid configuration value
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Backend not available (feature not enabled)
    #[error("Backend '{0}' is not available. Enable the corresponding feature flag.")]
    BackendNotAvailable(String),
}

impl From<http::method::InvalidMethod> for ConfigError {
    fn from(e: http::method::InvalidMethod) -> Self {
        ConfigError::InvalidMethod(String::new(), e)
    }
}

/// Parse YAML configuration from a string (without validation)
///
/// For enhanced validation with better error messages, use `validate_config` instead.
///
/// # Arguments
/// * `yaml` - YAML configuration string
///
/// # Examples
/// ```
/// use hitbox_configuration::ConfigEndpoint;
/// use hitbox_configuration::parse_config;
///
/// let yaml = r#"
/// policy:
///   Enabled:
///     ttl: 60
/// "#;
///
/// let config = parse_config::<ConfigEndpoint>(yaml)?;
/// # Ok::<(), hitbox_configuration::error::ConfigError>(())
/// ```
pub fn parse_config<T>(yaml: &str) -> Result<T, ConfigError>
where
    T: serde::de::DeserializeOwned,
{
    serde_saphyr::from_str(yaml).map_err(ConfigError::from)
}
