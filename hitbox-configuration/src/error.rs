use thiserror::Error;

/// Configuration error with enhanced location information
#[derive(Debug, Error)]
#[error("{}", display_with_context(.0, .1))]
pub struct ConfigError(#[source] serde_yaml::Error, Option<String>);

impl ConfigError {
    /// Create a new configuration error from a serde_yaml error
    pub fn from_yaml_error(error: serde_yaml::Error) -> Self {
        Self(error, None)
    }

    /// Create a configuration error with the original YAML content
    pub fn from_yaml_error_with_content(error: serde_yaml::Error, yaml_content: String) -> Self {
        Self(error, Some(yaml_content))
    }

    /// Get the line number where the error occurred (1-indexed)
    pub fn line(&self) -> Option<usize> {
        self.0.location().map(|loc| loc.line())
    }

    /// Get the column number where the error occurred (1-indexed)
    pub fn column(&self) -> Option<usize> {
        self.0.location().map(|loc| loc.column())
    }

    /// Get a formatted error message with location and context
    pub fn display_with_context(&self) -> String {
        display_with_context(&self.0, &self.1)
    }
}

/// Format the YAML content with context around the error location
/// Similar to hurl's error display with colors and markers
fn format_context(yaml: &str, error_line: usize, error_column: usize) -> String {
    format_context_with_color(yaml, error_line, error_column, should_use_color())
}

/// Check if we should use colored output (if stdout is a terminal)
fn should_use_color() -> bool {
    // Check environment variable first (allows forcing colors on/off)
    if let Ok(val) = std::env::var("NO_COLOR")
        && !val.is_empty()
    {
        return false;
    }

    if let Ok(val) = std::env::var("FORCE_COLOR")
        && !val.is_empty()
    {
        return true;
    }

    // Default to true for now (user can disable with NO_COLOR env var)
    // A more sophisticated check would use atty/isatty crate
    true
}

/// Format context with optional ANSI colors
fn format_context_with_color(
    yaml: &str,
    error_line: usize,
    error_column: usize,
    use_colors: bool,
) -> String {
    let lines: Vec<&str> = yaml.lines().collect();
    let mut output = String::new();

    if error_line == 0 || error_line > lines.len() {
        return output;
    }

    let line_idx = error_line - 1;
    let context_before = 2;
    let context_after = 2;

    // Determine the range of lines to show
    let start = line_idx.saturating_sub(context_before);
    let end = (line_idx + context_after + 1).min(lines.len());

    // Calculate line number width for alignment
    let max_line_num = end;
    let line_num_width = max_line_num.to_string().len();

    // ANSI color codes
    let blue = if use_colors { "\x1b[1;34m" } else { "" };
    let red = if use_colors { "\x1b[1;31m" } else { "" };
    let dim = if use_colors { "\x1b[2m" } else { "" };
    let reset = if use_colors { "\x1b[0m" } else { "" };

    // Location indicator (like hurl: --> line X, column Y)
    output.push_str(&format!(
        "  {blue}-->{reset} line {}, column {}\n",
        error_line,
        error_column,
        blue = blue,
        reset = reset
    ));

    for (idx, line) in lines.iter().enumerate().take(end).skip(start) {
        let line_num = idx + 1;
        let is_error_line = idx == line_idx;

        if is_error_line {
            // Error line - highlighted in red
            output.push_str(&format!(
                "{red}{:>width$}{reset} {blue}|{reset} {}\n",
                line_num,
                line,
                width = line_num_width,
                red = red,
                blue = blue,
                reset = reset
            ));

            // Add caret indicator at error column
            let marker_spaces = error_column.saturating_sub(1);
            output.push_str(&format!(
                "{:>width$} {blue}|{reset} {}{red}^{reset}\n",
                "",
                " ".repeat(marker_spaces),
                width = line_num_width,
                blue = blue,
                red = red,
                reset = reset
            ));
        } else {
            // Context line - dimmed
            output.push_str(&format!(
                "{dim}{:>width$}{reset} {blue}|{reset} {}\n",
                line_num,
                line,
                width = line_num_width,
                dim = dim,
                blue = blue,
                reset = reset
            ));
        }
    }

    output
}

/// Display error with context
fn display_with_context(error: &serde_yaml::Error, yaml_content: &Option<String>) -> String {
    let mut output = String::new();
    let use_colors = should_use_color();

    // ANSI color codes
    let red = if use_colors { "\x1b[1;31m" } else { "" };
    let reset = if use_colors { "\x1b[0m" } else { "" };

    // Show the error message with "error:" prefix in red
    output.push_str(&format!("{}error{}: {}\n", red, reset, error));

    // Add location and context if available
    if let Some(location) = error.location() {
        // Show context if yaml_content is available
        if let Some(yaml) = yaml_content {
            output.push('\n');
            output.push_str(&format_context(yaml, location.line(), location.column()));
        } else {
            // Fallback: just show location without context
            output.push_str(&format!(
                "  at line {}, column {}\n",
                location.line(),
                location.column()
            ));
        }
    }

    output
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(error: serde_yaml::Error) -> Self {
        Self::from_yaml_error(error)
    }
}

/// Parse YAML configuration from a string with enhanced error reporting
pub fn parse_config<T>(yaml: &str) -> Result<T, ConfigError>
where
    T: serde::de::DeserializeOwned,
{
    serde_yaml::from_str(yaml)
        .map_err(|err| ConfigError::from_yaml_error_with_content(err, yaml.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_location() {
        let yaml = r"
policy: !Enabled
  ttl: 5
response:
  - InvalidPredicate: 200
";

        let result = parse_config::<crate::ConfigEndpoint>(yaml);

        match result {
            Ok(_) => panic!("Expected error"),
            Err(err) => {
                assert!(err.line().is_some());
                assert!(err.column().is_some());

                let display = err.display_with_context();
                // New format uses "error:" prefix instead of "Configuration error:"
                assert!(display.contains("error"));
                assert!(display.contains("-->")); // Location indicator
                assert!(display.contains("|")); // Context border
            }
        }
    }

    #[test]
    fn test_config_error_without_content() {
        let yaml = r"
policy: !Enabled
  ttl: 5
response:
  - InvalidPredicate: 200
";

        let result: Result<crate::ConfigEndpoint, _> = serde_yaml::from_str(yaml);

        match result {
            Ok(_) => panic!("Expected error"),
            Err(err) => {
                let config_err = ConfigError::from_yaml_error(err);
                assert!(config_err.line().is_some());
                assert!(config_err.column().is_some());

                // Without content, no context borders should be shown
                let display = config_err.display_with_context();
                assert!(display.contains("error"));
                assert!(!display.contains("-->")); // No location indicator without content
            }
        }
    }
}
