use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Value {
    Scalar(String),
    Array(Vec<String>),
}

impl Value {
    pub fn inner(&self) -> Vec<String> {
        match self {
            Value::Scalar(value) => vec![value.to_owned()],
            Value::Array(values) => values.to_owned(),
        }
    }
}

pub fn parse(value: &str) -> HashMap<String, Value> {
    serde_qs::from_str(value).expect("Unreachable branch reached")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_one() {
        let hash_map = parse("key=value");
        let value = hash_map.get("key").unwrap();
        assert_eq!(value.inner(), vec!["value"]);
    }

    #[test]
    fn test_parse_valid_multiple() {
        let hash_map = parse("key-one=value-one&key-two=value-two&key-three=value-three");
        let value = hash_map.get("key-one").unwrap();
        assert_eq!(value.inner(), vec!["value-one"]);
        let value = hash_map.get("key-two").unwrap();
        assert_eq!(value.inner(), vec!["value-two"]);
        let value = hash_map.get("key-three").unwrap();
        assert_eq!(value.inner(), vec!["value-three"]);
    }

    #[test]
    fn test_parse_not_valid() {
        let hash_map = parse("   wrong   ");
        assert_eq!(hash_map.len(), 1);
    }
}
