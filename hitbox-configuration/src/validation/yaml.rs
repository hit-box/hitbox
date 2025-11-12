//! Custom YAML parser that preserves source location information.
//!
//! This module provides a YAML parser that tracks the source position (line and column)
//! of each element in the YAML document. This is used to map JSON Schema validation
//! errors to their exact location in the source YAML file.
//!
//! Based on Apollo Router's implementation:
//! https://github.com/apollographql/router/blob/dev/apollo-router/src/configuration/yaml.rs

use std::collections::HashMap;

use indexmap::IndexMap;
use jsonschema::paths::Location;
use yaml_rust::Event;
use yaml_rust::parser::MarkedEventReceiver;
use yaml_rust::parser::Parser;
use yaml_rust::scanner::Marker;

use crate::ConfigError;

/// A label with its source position
#[derive(Clone, Debug, Eq)]
pub(crate) struct Label {
    pub(crate) name: String,
    pub(crate) marker: Option<Marker>,
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl std::hash::Hash for Label {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl From<String> for Label {
    fn from(name: String) -> Self {
        Label { name, marker: None }
    }
}

/// A YAML value with source position information
#[derive(Clone, Debug)]
pub(crate) enum Value {
    String(String, Marker),
    Sequence(Vec<Value>, Marker),
    Mapping(Option<Label>, IndexMap<Label, Value>, Marker),
}

impl Value {
    /// Get the end marker for this value (useful for multi-line values)
    pub(crate) fn end_marker(&self) -> &Marker {
        match self {
            Value::String(_, m) => m,
            Value::Sequence(v, m) => v.last().map(|l| l.end_marker()).unwrap_or(m),
            Value::Mapping(_, v, m) => v
                .last()
                .map(|(_, val)| val.end_marker())
                .unwrap_or(m),
        }
    }

    /// Get the start marker for this value
    pub(crate) fn marker(&self) -> &Marker {
        match self {
            Value::String(_, m) => m,
            Value::Sequence(_, m) => m,
            Value::Mapping(_, _, m) => m,
        }
    }

    /// Extract the source text for this value
    pub(crate) fn source_text<'a>(&self, source: &'a str) -> &'a str {
        let start_marker = self.marker();
        let end_marker = self.end_marker();

        // Extract the text between start and end markers
        extract_source_between_markers(source, start_marker, end_marker)
    }
}

/// Extract source text between two markers
fn extract_source_between_markers<'a>(
    source: &'a str,
    start: &Marker,
    end: &Marker,
) -> &'a str {
    let lines: Vec<&str> = source.split('\n').collect();

    // Calculate byte offset to start
    let start_byte: usize = lines.iter()
        .take(start.line().saturating_sub(1))
        .map(|line| line.len() + 1) // +1 for newline
        .sum::<usize>()
        + start.col();

    // Calculate byte offset to end
    let end_byte: usize = lines.iter()
        .take(end.line().saturating_sub(1))
        .map(|line| line.len() + 1)
        .sum::<usize>()
        + end.col();

    let end_byte = end_byte.min(source.len());
    let start_byte = start_byte.min(source.len());

    if start_byte < end_byte {
        &source[start_byte..end_byte]
    } else {
        "" // Fallback for edge cases
    }
}

/// A YAML parser that preserves marker (position) information.
///
/// This parser builds an AST where each node knows its source location.
/// We can then use JSON Schema validation error paths to look up the
/// exact source location of errors.
#[derive(Default, Debug)]
pub(crate) struct MarkedYaml {
    anchors: HashMap<usize, Value>,
    current_label: Option<Label>,
    object_stack: Vec<(Option<Label>, Value, usize)>,
    root: Option<Value>,
}

impl MarkedYaml {
    /// Get an element by string path
    ///
    /// This is used to map error paths to YAML elements
    pub(crate) fn get_element_by_path(&self, path: &str) -> Option<&Value> {
        let mut current = self.root();
        if path.is_empty() || path == "/" {
            return current;
        }

        for segment_str in path.split('/').skip(1) {
            // Try to parse as array index first
            if let Ok(idx) = segment_str.parse::<usize>() {
                current = match current {
                    Some(Value::Sequence(sequence, _)) => sequence.get(idx),
                    _ => None,
                };
            } else {
                // It's a property name
                current = match current {
                    Some(Value::Mapping(_current_label, mapping, _)) => {
                        mapping.get(&Label::from(segment_str.to_string()))
                    }
                    _ => None,
                };
            }
        }
        current
    }

    /// Get an element by JSON pointer path
    ///
    /// This is used to map JSON Schema validation error paths to YAML elements
    pub(crate) fn get_element(&self, pointer: &Location) -> Option<&Value> {
        let pointer_str = pointer.to_string();
        self.get_element_by_path(&pointer_str)
    }

    #[allow(dead_code)]
    fn get_element_old(&self, pointer: &Location) -> Option<&Value> {
        let mut current = self.root();
        // Convert pointer to string and parse it
        // JSON pointers look like "/path/to/field" or "/array/0/field"
        let pointer_str = pointer.to_string();
        if pointer_str.is_empty() || pointer_str == "/" {
            return current;
        }

        for segment_str in pointer_str.split('/').skip(1) {
            // Try to parse as array index first
            if let Ok(idx) = segment_str.parse::<usize>() {
                current = match current {
                    Some(Value::Sequence(sequence, _)) => sequence.get(idx),
                    _ => None,
                };
            } else {
                // It's a property name
                current = match current {
                    Some(Value::Mapping(_current_label, mapping, _)) => {
                        mapping.get(&Label::from(segment_str.to_string()))
                    }
                    _ => None,
                };
            }
        }
        current
    }

    fn root(&self) -> Option<&Value> {
        self.root.as_ref()
    }

    fn end_container(&mut self) -> Option<Value> {
        let (label, v, id) = self.object_stack.pop().expect("imbalanced parse events");
        self.anchors.insert(id, v.clone());
        match (label, self.object_stack.last_mut()) {
            (Some(label), Some((_, Value::Mapping(_current_label, mapping, _), _))) => {
                mapping.insert(label, v);
                None
            }
            (None, Some((_, Value::Sequence(sequence, _), _))) => {
                sequence.push(v);
                None
            }
            _ => Some(v),
        }
    }

    fn add_value(&mut self, marker: Marker, v: String, id: usize) {
        match (self.current_label.take(), self.object_stack.last_mut()) {
            (Some(label), Some((_, Value::Mapping(_current_label, mapping, _), _))) => {
                let v = Value::String(v, marker);
                self.anchors.insert(id, v.clone());
                mapping.insert(label, v);
            }
            (None, Some((_, Value::Sequence(sequence, _), _))) => {
                let v = Value::String(v, marker);
                self.anchors.insert(id, v.clone());
                sequence.push(v);
            }
            (None, _) => {
                self.current_label = Some(Label {
                    name: v,
                    marker: Some(marker),
                })
            }
            _ => {
                // Labeled scalar without container
            }
        }
    }

    fn add_alias_value(&mut self, v: Value) {
        match (self.current_label.take(), self.object_stack.last_mut()) {
            (Some(label), Some((_, Value::Mapping(_current_label, mapping, _), _))) => {
                mapping.insert(label, v);
            }
            (None, Some((_, Value::Sequence(sequence, _), _))) => {
                sequence.push(v);
            }
            _ => {
                // Scalar without container
            }
        }
    }
}

/// Parse YAML with position tracking
pub(crate) fn parse(source: &str) -> Result<MarkedYaml, ConfigError> {
    // Yaml parser doesn't support CRLF. Remove CRs.
    let source = source.replace('\r', "");
    let mut parser = Parser::new(source.chars());
    let mut loader = MarkedYaml::default();
    parser
        .load(&mut loader, true)
        .map_err(|e| ConfigError::YamlScan(e.to_string()))?;

    Ok(loader)
}

impl MarkedEventReceiver for MarkedYaml {
    fn on_event(&mut self, ev: Event, marker: Marker) {
        match ev {
            Event::Scalar(v, _style, id, _tag) => self.add_value(marker, v, id),
            Event::SequenceStart(id) => {
                self.object_stack.push((
                    self.current_label.take(),
                    Value::Sequence(Vec::new(), marker),
                    id,
                ));
            }
            Event::SequenceEnd => {
                self.root = self.end_container();
            }
            Event::MappingStart(id) => {
                let current_label = self.current_label.take();
                self.object_stack.push((
                    current_label.clone(),
                    Value::Mapping(current_label, IndexMap::default(), marker),
                    id,
                ));
            }
            Event::MappingEnd => {
                self.root = self.end_container();
            }
            Event::Alias(id) => {
                if let Some(v) = self.anchors.get(&id) {
                    let cloned = v.clone();
                    self.add_alias_value(cloned);
                }
            }
            Event::DocumentStart => {}
            Event::DocumentEnd => {}
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let yaml = r#"
policy:
  Enabled:
    ttl: 60
    stale: 30
"#;
        let parsed = parse(yaml).unwrap();
        let root = parsed.root().unwrap();

        // Should be able to traverse the structure
        match root {
            Value::Mapping(_, map, _) => {
                assert!(map.contains_key(&Label::from("policy".to_string())));
            }
            _ => panic!("Expected mapping at root"),
        }
    }

    #[test]
    fn test_get_element_by_path() {
        let yaml = r#"
request:
  - Method: "GET"
response:
  - Status:
      class: Success
policy:
  Enabled:
    ttl: 60
"#;
        let parsed = parse(yaml).unwrap();

        // Test getting nested element using a path string
        // Note: In production, this path comes from jsonschema::ValidationError.instance_path
        // We can't easily construct a Location manually as join() is private,
        // but we can test with a simple path
        let root_element = parsed.get_element(&Location::new());
        assert!(root_element.is_some());

        // Verify we can access the root structure
        match root_element {
            Some(Value::Mapping(_, map, _)) => {
                assert!(map.contains_key(&Label::from("policy".to_string())));
                assert!(map.contains_key(&Label::from("request".to_string())));
                assert!(map.contains_key(&Label::from("response".to_string())));
            }
            _ => panic!("Expected mapping at root"),
        }
    }
}
