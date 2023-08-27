use http::Method;
use serde::{self, Deserialize, Deserializer, Serializer};

pub fn serialize<S>(method: &Method, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = method.to_string();
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Method, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Method::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom)
}
