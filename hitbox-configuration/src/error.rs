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

    /// Invalid HTTP version
    #[error("Invalid HTTP version: {0}")]
    InvalidVersion(String),

    /// Invalid regex pattern
    #[error("Invalid regex pattern '{pattern}': {error}")]
    InvalidRegex {
        pattern: String,
        #[source]
        error: regex::Error,
    },

    /// Invalid predicate configuration
    #[error("Invalid predicate: {0}")]
    InvalidPredicate(String),

    /// Backend not available (feature not enabled)
    #[error("Backend '{0}' is not available. Enable the corresponding feature flag.")]
    BackendNotAvailable(String),

    /// Empty path list in 'in' operation
    #[error("Path 'in' operation requires at least one pattern")]
    EmptyPathList,

    /// Error originating from a specific endpoint in a multi-endpoint configuration
    ///
    /// The message names the endpoint only; the underlying error is available
    /// through [`std::error::Error::source`], so error reporters print it once.
    #[error("Invalid endpoint at index {index}{}", endpoint_name(.name))]
    EndpointAt {
        /// Zero-based position of the failing endpoint in the list
        index: usize,
        /// Name of the failing endpoint, when one was provided
        name: Option<String>,
        #[source]
        source: Box<ConfigError>,
    },
}

/// Formats an optional endpoint name as a ``" (`name`)"`` suffix for [`ConfigError::EndpointAt`].
fn endpoint_name(name: &Option<String>) -> String {
    name.as_deref()
        .map(|name| format!(" (`{name}`)"))
        .unwrap_or_default()
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
