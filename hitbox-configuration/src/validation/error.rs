//! Error collection, scoring, and improvement logic

use super::formatting::{format_suberror_message, format_validation_error};

/// A collected error with full path information for scoring and display
#[derive(Debug, Clone)]
pub(super) struct CollectedError {
    /// Full instance path (including parent paths from anyOf)
    pub(super) instance_path: String,
    /// Error kind for scoring
    pub(super) kind: String,
    /// Human-readable error message
    pub(super) message: String,
    /// Path depth for scoring
    pub(super) depth: usize,
}

/// Improve error messages for unknown/misspelled property names
pub(super) fn improve_error_for_unknown_properties(error: &mut CollectedError, instance: &serde_json::Value) {
    // Only process "Required" errors
    if !error.kind.contains("Required") {
        return;
    }

    // Try to find the instance at the error path
    let instance_at_path = navigate_to_instance_by_string_path(instance, &error.instance_path);
    if instance_at_path.is_none() {
        return;
    }

    let instance_obj = instance_at_path.unwrap();
    if !instance_obj.is_object() {
        return;
    }

    // Get the actual properties in the instance
    let actual_properties: Vec<String> = instance_obj
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.to_string())
        .collect();

    // Known valid properties for request/response predicates and policy
    let valid_properties = ["And", "Or", "Status", "Body", "Header", "Method", "Path", "Query", "Enabled", "Disabled"];

    // Check if any actual property is not in the valid list
    for prop in &actual_properties {
        if !valid_properties.contains(&prop.as_str()) {
            // Try to find the closest match using Levenshtein distance
            let mut closest_match: Option<(&str, usize)> = None;

            for &valid_prop in &valid_properties {
                let distance = strsim::levenshtein(prop, valid_prop);
                // Consider it a potential typo if edit distance is 1 or 2 (depending on length)
                let threshold = if prop.len() > 3 { 2 } else { 1 };

                if distance <= threshold {
                    if let Some((_, best_distance)) = closest_match {
                        if distance < best_distance {
                            closest_match = Some((valid_prop, distance));
                        }
                    } else {
                        closest_match = Some((valid_prop, distance));
                    }
                }
            }

            // Update error message based on whether we found a close match
            if let Some((suggestion, _)) = closest_match {
                error.message = format!("unknown property '{}', did you mean '{}'?", prop, suggestion);
            } else {
                error.message = format!("unknown property '{}'", prop);
            }
            return;
        }
    }
}

/// Navigate to a value in the instance using a string path like "/response" or "/response/0"
fn navigate_to_instance_by_string_path<'a>(
    instance: &'a serde_json::Value,
    path_str: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = instance;

    // Instance paths look like: /response or /response/0/Status
    for segment in path_str.split('/').filter(|s| !s.is_empty()) {
        // Try to parse as array index
        if let Ok(idx) = segment.parse::<usize>() {
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }

    Some(current)
}

/// Collect all validation errors by directly validating against each anyOf branch
pub(super) fn collect_all_errors_recursively<'a>(
    errors: &[jsonschema::ValidationError<'a>],
    schema: &serde_json::Value,
    instance: &serde_json::Value,
) -> Vec<CollectedError> {
    let mut all_errors = Vec::new();

    for error in errors {
        // Add the top-level error
        let path_str = error.instance_path.to_string();
        all_errors.push(CollectedError {
            instance_path: path_str.clone(),
            kind: format!("{:?}", error.kind),
            message: format_validation_error(error, None),
            depth: path_str.matches('/').count(),
        });

        // If it's an anyOf or oneOf error, validate against each branch and collect ALL errors
        use jsonschema::error::ValidationErrorKind;
        if matches!(error.kind, ValidationErrorKind::AnyOf | ValidationErrorKind::OneOfNotValid | ValidationErrorKind::OneOfMultipleValid) {
            collect_anyof_branch_errors(error, "", schema, instance, &mut all_errors);
        }
    }

    all_errors
}

/// Directly validate instance against each anyOf branch and collect all errors
fn collect_anyof_branch_errors(
    error: &jsonschema::ValidationError,
    accumulated_parent_path: &str,
    schema: &serde_json::Value,
    full_instance: &serde_json::Value,
    collected: &mut Vec<CollectedError>,
) {
    // Navigate to the schema
    let schema_at_path = match navigate_to_schema_path(schema, &error.schema_path) {
        Some(s) => s,
        None => return,
    };

    let anyof_schemas = if let Some(arr) = schema_at_path.as_array() {
        arr
    } else if let Some(arr) = schema_at_path.get("anyOf").and_then(|v| v.as_array()) {
        arr
    } else if let Some(arr) = schema_at_path.get("oneOf").and_then(|v| v.as_array()) {
        arr
    } else {
        return;
    };

    // Get the instance at the error path
    let instance = match navigate_to_instance_path(full_instance, &error.instance_path) {
        Some(i) => i,
        None => return,
    };

    let definitions = schema.get("definitions").cloned();

    // Build the accumulated parent path for this level
    let error_path_str = error.instance_path.to_string();
    let current_parent = if accumulated_parent_path.is_empty() {
        error_path_str.clone()
    } else if error_path_str.is_empty() || error_path_str == "/" {
        accumulated_parent_path.to_string()
    } else {
        format!("{}{}", accumulated_parent_path, error_path_str)
    };

    // Validate against each anyOf branch
    for sub_schema in anyof_schemas.iter() {
        let resolved_schema = match resolve_ref(sub_schema, schema) {
            Some(s) => s,
            None => continue,
        };

        let schema_with_definitions = add_definitions_to_schema(resolved_schema, definitions.as_ref());

        // Create validator and collect ALL errors from this branch
        if let Ok(validator) = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .build(&schema_with_definitions) {

            let branch_errors: Vec<_> = validator.iter_errors(instance).collect();

            for sub_error in branch_errors {
                let sub_path = sub_error.instance_path.to_string();
                let full_path = if sub_path.is_empty() || sub_path == "/" {
                    current_parent.clone()
                } else {
                    format!("{}{}", current_parent, sub_path)
                };

                collected.push(CollectedError {
                    instance_path: full_path.clone(),
                    kind: format!("{:?}", sub_error.kind),
                    message: format_suberror_message(&sub_error),
                    depth: full_path.matches('/').count(),
                });

                // If this sub-error is anyOf or oneOf, recursively collect from it
                use jsonschema::error::ValidationErrorKind;
                if matches!(sub_error.kind, ValidationErrorKind::AnyOf | ValidationErrorKind::OneOfNotValid | ValidationErrorKind::OneOfMultipleValid) {
                    collect_anyof_branch_errors(&sub_error, &current_parent, &schema_with_definitions, instance, collected);
                }
            }
        }
    }
}

/// Find the best match error from collected errors
pub(super) fn find_best_match_collected(errors: &[CollectedError]) -> &CollectedError {
    let mut best = &errors[0];
    let mut best_score = calculate_collected_error_weight(best);

    for error in errors.iter().skip(1) {
        let score = calculate_collected_error_weight(error);
        if score > best_score {
            best = error;
            best_score = score;
        }
    }

    best
}

/// Calculate weight for a collected error
fn calculate_collected_error_weight(error: &CollectedError) -> i32 {
    let mut weight = 0;

    // Path depth: deeper is more specific
    weight += (error.depth as i32) * 10;

    // Error kind scoring
    if error.kind.contains("AnyOf") {
        weight -= 100;
    } else if error.kind.contains("Required") {
        weight += 50;
    } else if error.kind.contains("Enum") {
        weight += 45;
    } else if error.kind.contains("Type") {
        weight += 40;
    } else if error.kind.contains("Pattern") {
        weight += 35;
    } else if error.kind.contains("Minimum") || error.kind.contains("Maximum") {
        weight += 30;
    } else if error.kind.contains("AdditionalProperties") {
        weight += 25;
    } else {
        weight += 20;
    }

    weight
}

/// Resolve a $ref in a schema
fn resolve_ref<'a>(
    schema_value: &'a serde_json::Value,
    root_schema: &'a serde_json::Value,
) -> Option<&'a serde_json::Value> {
    if let Some(ref_str) = schema_value.get("$ref").and_then(|v| v.as_str()) {
        if let Some(stripped) = ref_str.strip_prefix("#/") {
            let mut current = root_schema;
            for segment in stripped.split('/') {
                current = current.get(segment)?;
            }
            return Some(current);
        }
    }
    Some(schema_value)
}

/// Add definitions to a schema for $ref resolution
fn add_definitions_to_schema(
    schema: &serde_json::Value,
    definitions: Option<&serde_json::Value>,
) -> serde_json::Value {
    if let (Some(defs), serde_json::Value::Object(mut obj)) = (definitions, schema.clone()) {
        obj.insert("definitions".to_string(), defs.clone());
        serde_json::Value::Object(obj)
    } else if let Some(defs) = definitions {
        serde_json::json!({
            "allOf": [schema],
            "definitions": defs
        })
    } else {
        schema.clone()
    }
}

/// Navigate to a specific instance value using a path
fn navigate_to_instance_path<'a>(
    instance: &'a serde_json::Value,
    path: &jsonschema::paths::Location,
) -> Option<&'a serde_json::Value> {
    let mut current = instance;
    let path_str = path.to_string();

    // Instance paths look like: /response or /response/0/Status
    for segment in path_str.split('/').filter(|s| !s.is_empty()) {
        // Try to parse as array index
        if let Ok(idx) = segment.parse::<usize>() {
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }

    Some(current)
}

/// Navigate to a specific location in the schema using a path
fn navigate_to_schema_path<'a>(
    schema: &'a serde_json::Value,
    path: &jsonschema::paths::Location,
) -> Option<&'a serde_json::Value> {
    let mut current = schema;
    let path_str = path.to_string();

    // Schema paths look like: /properties/response/allOf/0/$ref/anyOf
    // When we see $ref, we need to resolve it
    for segment in path_str.split('/').filter(|s| !s.is_empty()) {
        // If we hit $ref, resolve the reference
        if segment == "$ref" {
            // Current should have a $ref key
            if let Some(ref_str) = current.get("$ref").and_then(|v| v.as_str()) {
                // Reference looks like "#/definitions/Nullable_Response"
                if let Some(stripped) = ref_str.strip_prefix("#/") {
                    // Navigate to the definition
                    current = schema; // Start from root
                    for ref_segment in stripped.split('/') {
                        current = current.get(ref_segment)?;
                    }
                    continue;
                }
            }
            return None;
        }

        // Try to parse as array index
        if let Ok(idx) = segment.parse::<usize>() {
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }

    Some(current)
}
