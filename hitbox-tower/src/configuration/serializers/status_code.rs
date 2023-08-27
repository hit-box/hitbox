use http::StatusCode;
use serde::{self, Deserialize, Deserializer, Serializer};

pub fn serialize<S>(status_code: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = status_code.to_string();
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    StatusCode::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom)
}
