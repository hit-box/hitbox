//! JSON Schema generation and validation

use crate::error::ConfigError;
use super::error::{collect_all_errors_recursively, find_best_match_collected, improve_error_for_unknown_properties};
use super::formatting::format_error_context;
use super::yaml;

/// Validate YAML configuration against JSON Schema with enhanced error messages
pub(super) fn validate_with_schema<T>(yaml: &str) -> Result<T, ConfigError>
where
    T: serde::de::DeserializeOwned + schemars::JsonSchema,
{
    // First parse YAML to JSON Value
    let value: serde_json::Value = serde_saphyr::from_str(yaml)?;

    // Generate schema for type T using Draft07
    let generator = schemars::r#gen::SchemaSettings::draft07().into_generator();
    let schema = generator.into_root_schema_for::<T>();
    let schema_value = serde_json::to_value(&schema)?;

    // Compile and validate against schema using Draft07
    let compiled_schema = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .build(&schema_value)
        .map_err(|e| ConfigError::Validation(format!("Failed to compile schema: {}", e)))?;

    // Validate the configuration - collect errors to release borrow on value
    let validation_errors: Vec<_> = compiled_schema.iter_errors(&value).collect();

    if !validation_errors.is_empty() {
        // Collect all nested errors from anyOf/oneOf branches recursively
        // This flattens the error tree so we can find the most specific error
        let all_errors = collect_all_errors_recursively(&validation_errors, &schema_value, &value);

        // Find the best match error (most relevant/specific) from ALL errors
        let mut best_error = find_best_match_collected(&all_errors).clone();

        // Improve error message for typos in property names
        improve_error_for_unknown_properties(&mut best_error, &value);

        // Parse YAML with position tracking for better error messages
        let parsed_yaml = yaml::parse(yaml)?;
        let yaml_lines: Vec<&str> = yaml.split('\n').collect();

        // Show only the best error to keep output focused
        let error_message = if let Some(element) = parsed_yaml.get_element_by_path(&best_error.instance_path) {
            let marker = element.marker();
            let end_marker = element.end_marker();

            // Format error with context lines (Rust compiler style)
            let context = format_error_context(&yaml_lines, marker, end_marker, &best_error.message);

            format!(
                "  --> configuration:{}:{}\n{}\n",
                marker.line(),
                marker.col(),
                context
            )
        } else {
            // Fallback to basic formatting if element not found
            let path_str = if best_error.instance_path.is_empty() {
                "root".to_string()
            } else {
                best_error.instance_path.clone()
            };
            format!("  --> {}\n   | {}\n", path_str, best_error.message)
        };

        return Err(ConfigError::Validation(error_message));
    }

    // If validation passes, deserialize
    serde_json::from_value(value).map_err(ConfigError::from)
}

/// Generate JSON Schema for a configuration type (Draft 07)
pub(super) fn generate_schema<T>() -> schemars::schema::RootSchema
where
    T: schemars::JsonSchema,
{
    let generator = schemars::r#gen::SchemaSettings::draft07().into_generator();
    generator.into_root_schema_for::<T>()
}
