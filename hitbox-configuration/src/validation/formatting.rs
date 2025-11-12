//! Error message formatting with Rust compiler style

use yaml_rust::scanner::Marker;

/// Format a TypeKind into a human-readable string
pub(super) fn format_type_kind(type_kind: &jsonschema::error::TypeKind) -> String {
    use jsonschema::error::TypeKind;

    match type_kind {
        TypeKind::Single(ptype) => {
            // Single primitive type
            format_primitive_type_name(ptype)
        }
        TypeKind::Multiple(types_bitmap) => {
            // Multiple types in a bitmap - clone and iterate
            let type_names: Vec<String> = types_bitmap
                .clone()
                .into_iter()
                .map(|pt| format_primitive_type_name(&pt))
                .collect();

            // Format as "type1, type2, or type3"
            match type_names.len() {
                0 => "unknown type".to_string(),
                1 => type_names[0].clone(),
                2 => format!("{} or {}", type_names[0], type_names[1]),
                _ => {
                    let mut names = type_names;
                    let last = names.pop().unwrap();
                    format!("{}, or {}", names.join(", "), last)
                }
            }
        }
    }
}

/// Format a single PrimitiveType name
fn format_primitive_type_name(ptype: &jsonschema::primitive_type::PrimitiveType) -> String {
    use jsonschema::primitive_type::PrimitiveType;

    match ptype {
        PrimitiveType::Null => "null".to_string(),
        PrimitiveType::Boolean => "boolean".to_string(),
        PrimitiveType::Object => "object".to_string(),
        PrimitiveType::Array => "array".to_string(),
        PrimitiveType::Number => "number".to_string(),
        PrimitiveType::String => "string".to_string(),
        PrimitiveType::Integer => "integer".to_string(),
    }
}

/// Format a sub-error message concisely
pub(super) fn format_suberror_message(error: &jsonschema::ValidationError) -> String {
    use jsonschema::error::ValidationErrorKind;

    match &error.kind {
        ValidationErrorKind::Required { property } => {
            format!("missing required property '{}'", property)
        }
        ValidationErrorKind::Type { kind } => {
            let type_str = format_type_kind(kind);
            format!("wrong type (expected {})", type_str)
        }
        ValidationErrorKind::Enum { options } => {
            format!("value must be one of {}", options)
        }
        ValidationErrorKind::Pattern { pattern } => {
            format!("must match pattern '{}'", pattern)
        }
        ValidationErrorKind::Minimum { limit } | ValidationErrorKind::ExclusiveMinimum { limit } => {
            format!("must be >= {}", limit)
        }
        ValidationErrorKind::Maximum { limit } | ValidationErrorKind::ExclusiveMaximum { limit } => {
            format!("must be <= {}", limit)
        }
        ValidationErrorKind::AnyOf => {
            // For nested anyOf errors, provide a clean message without JSON
            let path = error.instance_path.to_string();
            if path.is_empty() {
                "value does not match any of the expected schemas".to_string()
            } else {
                format!("value at '{}' does not match any of the expected schemas", path)
            }
        }
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            format!("unexpected properties: {}", unexpected.join(", "))
        }
        _ => {
            // For other errors, show the message but strip out JSON representations
            let msg = error.to_string();
            // If the message contains large JSON objects/arrays, simplify it
            if msg.len() > 100 && (msg.contains('{') || msg.contains('[')) {
                // Extract just the validation constraint, not the value
                if let Some(pos) = msg.find(" is not ") {
                    format!("value{}", &msg[pos..])
                } else {
                    "validation failed".to_string()
                }
            } else {
                msg
            }
        }
    }
}

/// Format error context lines in Rust compiler style
pub(super) fn format_error_context(
    yaml_lines: &[&str],
    start_marker: &Marker,
    end_marker: &Marker,
    error_message: &str,
) -> String {
    const NUM_CONTEXT_LINES: usize = 2;

    let start_line = start_marker.line().saturating_sub(1); // Convert to 0-indexed
    let end_line = end_marker.line().saturating_sub(1).min(yaml_lines.len().saturating_sub(1));
    let offset = start_line.saturating_sub(NUM_CONTEXT_LINES);

    // Calculate the max line number width for alignment
    let max_line_num = end_line + 1;
    let line_num_width = max_line_num.to_string().len();

    let mut output = Vec::new();

    // Add separator line
    output.push(format!("{:width$} |", "", width = line_num_width));

    // Add context lines
    for (idx, line) in yaml_lines[offset..=end_line].iter().enumerate() {
        let real_line = idx + offset;
        let line_num = real_line + 1; // Display as 1-indexed

        output.push(format!("{:>width$} | {}", line_num, line, width = line_num_width));

        // Add underline and error message on the error line
        if real_line == start_line {
            let start_col = start_marker.col();

            // Calculate end column - if multi-line, use end of current line
            let end_col = if start_line == end_line {
                end_marker.col()
            } else {
                line.len()
            };

            // Calculate underline length, minimum 1 character
            let underline_len = if end_col > start_col {
                end_col - start_col
            } else {
                // If end_col <= start_col, try to underline at least the value
                // by finding the extent from the start position
                let remaining = &line[start_col..];
                // Find the length of the value (up to next space or end of line)
                remaining.find(char::is_whitespace).unwrap_or(remaining.len()).max(1)
            };

            let underline = "^".repeat(underline_len);
            output.push(format!(
                "{:width$} | {:indent$}{} {}",
                "",
                "",
                underline,
                error_message,
                width = line_num_width,
                indent = start_col
            ));
        }
    }

    output.join("\n")
}

/// Format validation error message with source YAML value instead of JSON
pub(super) fn format_validation_error(error: &jsonschema::ValidationError, source_value: Option<&str>) -> String {
    use jsonschema::error::ValidationErrorKind;

    // Try to create a better message for common error types
    match &error.kind {
        ValidationErrorKind::Required { property } => {
            format!("{} is a required property", property)
        }
        ValidationErrorKind::Type { kind } if source_value.is_some() => {
            let type_str = format_type_kind(kind);
            format!("\"{}\" is not of type {}", source_value.unwrap().trim(), type_str)
        }
        ValidationErrorKind::Type { kind } => {
            let type_str = format_type_kind(kind);
            format!("wrong type (expected {})", type_str)
        }
        ValidationErrorKind::Enum { options } if source_value.is_some() => {
            format!("\"{}\" is not one of {}", source_value.unwrap().trim(), options)
        }
        ValidationErrorKind::AnyOf => {
            // For complex types (arrays, objects), don't show the value
            "value is not valid under any of the expected schemas".to_string()
        }
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            format!("Additional properties are not allowed ('{}' was/were unexpected)", unexpected.join(", "))
        }
        _ => {
            // For other error types, check if message contains JSON and clean it up
            let msg = error.to_string();
            // If the message contains JSON representations, try to simplify
            if (msg.contains('{') || msg.contains('[')) && msg.contains("oneOf") {
                "value is not valid under any of the schemas listed in the 'oneOf' keyword".to_string()
            } else {
                msg
            }
        }
    }
}
