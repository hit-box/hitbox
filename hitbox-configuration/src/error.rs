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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_basic() {
        // Test basic error wrapping
        let yaml = r"
policy:
  Enabled:
    ttl: not_a_number
request:
  - Method: GET
";

        let result = parse_config::<crate::ConfigEndpoint>(yaml);
        assert!(result.is_err(), "Expected error for invalid YAML");
    }

    #[test]
    fn test_config_error_from_conversion() {
        // Test From<serde_saphyr::Error> conversion
        let yaml = r"
policy:
  Enabled:
    ttl: not_a_number
request:
  - Method: GET
";

        let result: Result<crate::ConfigEndpoint, _> = serde_saphyr::from_str(yaml);
        match result {
            Ok(_) => panic!("Expected error"),
            Err(err) => {
                let _config_err: ConfigError = err.into();
                // Just verify the conversion works
            }
        }
    }
}
