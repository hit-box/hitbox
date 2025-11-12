//! Configuration validation with JSON Schema and enhanced error messages
//!
//! This module provides validation functionality with detailed, Rust-compiler-style
//! error messages including source location, typo suggestions, and context.

mod error;
mod formatting;
mod schema;
mod yaml;

use crate::error::ConfigError;

/// Validate YAML configuration against JSON Schema and parse
///
/// This function provides enhanced validation with:
/// - JSON Schema validation (Draft 07)
/// - Rust-compiler-style error messages
/// - Source location tracking (line:column)
/// - Typo detection and suggestions
/// - Context lines around errors
///
/// # Arguments
/// * `yaml` - YAML configuration string
///
/// # Examples
/// ```
/// use hitbox_configuration::ConfigEndpoint;
/// # #[cfg(feature = "validation")]
/// use hitbox_configuration::validation::validate_config;
///
/// # #[cfg(feature = "validation")]
/// # fn example() -> Result<(), hitbox_configuration::error::ConfigError> {
/// let yaml = r#"
/// policy:
///   Enabled:
///     ttl: 60
/// "#;
///
/// let config = validate_config::<ConfigEndpoint>(yaml)?;
/// # Ok(())
/// # }
/// ```
pub fn validate_config<T>(yaml: &str) -> Result<T, ConfigError>
where
    T: serde::de::DeserializeOwned + schemars::JsonSchema,
{
    schema::validate_with_schema(yaml)
}

/// Generate JSON Schema for a configuration type (Draft 07)
///
/// # Examples
/// ```
/// use hitbox_configuration::ConfigEndpoint;
/// # #[cfg(feature = "validation")]
/// use hitbox_configuration::validation::generate_schema;
///
/// # #[cfg(feature = "validation")]
/// # fn example() {
/// let schema = generate_schema::<ConfigEndpoint>();
/// let json = serde_json::to_string_pretty(&schema).unwrap();
/// println!("{}", json);
/// # }
/// ```
pub fn generate_schema<T>() -> schemars::schema::RootSchema
where
    T: schemars::JsonSchema,
{
    schema::generate_schema::<T>()
}
