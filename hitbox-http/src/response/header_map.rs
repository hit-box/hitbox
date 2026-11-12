//! Serde for the [`HeaderMap`] fields of [`SerializableHttpResponse`](super::SerializableHttpResponse).
//!
//! Binary formats use [`http_serde`]'s layout unchanged: a map from each name to a sequence of
//! byte strings.
//!
//! Text formats (JSON, RON) write what `http_serde` writes for ASCII values: a
//! string for a single value, a sequence of strings for several. `http_serde` drops any other
//! value; here such a value is written as an array of bytes, so a single one becomes a sequence
//! holding one array:
//!
//! ```json
//! {"content-type": "text/plain", "set-cookie": ["a=1", "b=2"], "x-name": [[99, 97, 102, 233]]}
//! ```
//!
//! Reading accepts everything `http_serde` wrote.

use std::fmt;

use http::header::GetAll;
use http::{HeaderMap, HeaderName, HeaderValue};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};

pub(super) fn serialize<S: Serializer>(
    headers: &HeaderMap,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if !serializer.is_human_readable() {
        return http_serde::header_map::serialize(headers, serializer);
    }
    serializer.collect_map(
        headers
            .keys()
            .map(|name| (name.as_str(), TextValues(headers.get_all(name)))),
    )
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<HeaderMap, D::Error> {
    if !deserializer.is_human_readable() {
        return http_serde::header_map::deserialize(deserializer);
    }
    deserializer.deserialize_map(TextHeaderMapVisitor)
}

/// All values of one header, written for a text format.
struct TextValues<'a>(GetAll<'a, HeaderValue>);

impl Serialize for TextValues<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut values = self.0.iter();
        if let (Some(value), None) = (values.next(), values.next())
            && let Ok(text) = value.to_str()
        {
            return serializer.serialize_str(text);
        }
        serializer.collect_seq(self.0.iter().map(TextValue))
    }
}

/// One header value in a sequence: a string, or an array of bytes if it has non-ASCII bytes (obs-text).
struct TextValue<'a>(&'a HeaderValue);

impl Serialize for TextValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0.to_str() {
            Ok(text) => serializer.serialize_str(text),
            // Numbers rather than `serialize_bytes`, whose text form depends on the format: bytes
            // written as a string would read back as the value itself.
            Err(_) => serializer.collect_seq(self.0.as_bytes()),
        }
    }
}

struct TextHeaderMapVisitor;

impl<'de> Visitor<'de> for TextHeaderMapVisitor {
    type Value = HeaderMap;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map of header names to values")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<HeaderMap, M::Error> {
        let mut headers = HeaderMap::try_with_capacity(access.size_hint().unwrap_or(0))
            .map_err(de::Error::custom)?;
        while let Some((name, TextValuesIn(values))) =
            access.next_entry::<String, TextValuesIn>()?
        {
            let name = HeaderName::from_bytes(name.as_bytes()).map_err(de::Error::custom)?;
            for value in values {
                let value = HeaderValue::try_from(value).map_err(de::Error::custom)?;
                headers
                    .try_append(&name, value)
                    .map_err(de::Error::custom)?;
            }
        }
        Ok(headers)
    }
}

/// All values of one header, read from a text format: a string, or a sequence of values.
struct TextValuesIn(Vec<Vec<u8>>);

impl<'de> Deserialize<'de> for TextValuesIn {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(TextValuesVisitor)
    }
}

struct TextValuesVisitor;

impl<'de> Visitor<'de> for TextValuesVisitor {
    type Value = TextValuesIn;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a header value or a sequence of header values")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(TextValuesIn(vec![value.as_bytes().to_vec()]))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(TextValuesIn(vec![value.into_bytes()]))
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        Ok(TextValuesIn(vec![value.to_vec()]))
    }

    fn visit_byte_buf<E: de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        Ok(TextValuesIn(vec![value]))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(TextValueIn(value)) = seq.next_element()? {
            values.push(value);
        }
        Ok(TextValuesIn(values))
    }
}

/// One header value in a sequence, read from a text format: a string or an array of bytes.
struct TextValueIn(Vec<u8>);

impl<'de> Deserialize<'de> for TextValueIn {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(TextValueVisitor)
    }
}

struct TextValueVisitor;

impl<'de> Visitor<'de> for TextValueVisitor {
    type Value = TextValueIn;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a header value as a string or an array of bytes")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(TextValueIn(value.as_bytes().to_vec()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(TextValueIn(value.into_bytes()))
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        Ok(TextValueIn(value.to_vec()))
    }

    fn visit_byte_buf<E: de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        Ok(TextValueIn(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut bytes = Vec::new();
        while let Some(byte) = seq.next_element::<u8>()? {
            bytes.push(byte);
        }
        Ok(TextValueIn(bytes))
    }
}

#[cfg(test)]
mod tests {
    use bincode::config::standard;
    use bincode::serde::encode_to_vec;
    use http::{HeaderMap, HeaderValue};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Serialize, Deserialize, Debug)]
    struct Headers(#[serde(with = "super")] HeaderMap);

    #[derive(Serialize, Deserialize, Debug)]
    struct HttpSerdeHeaders(#[serde(with = "http_serde::header_map")] HeaderMap);

    fn ascii() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/plain"));
        headers.append("set-cookie", HeaderValue::from_static("a=1"));
        headers.append("set-cookie", HeaderValue::from_static("b=2"));
        headers
    }

    fn non_ascii() -> HeaderMap {
        let mut headers = ascii();
        headers.insert("x-name", HeaderValue::from_bytes(b"caf\xe9").unwrap());
        headers.append("set-cookie", HeaderValue::from_bytes(b"c=\xe9").unwrap());
        headers
    }

    #[test]
    fn text_formats_write_ascii_like_http_serde() {
        assert_eq!(
            serde_json::to_value(Headers(ascii())).unwrap(),
            serde_json::to_value(HttpSerdeHeaders(ascii())).unwrap(),
        );
    }

    #[test]
    fn text_formats_write_other_values_as_byte_arrays() {
        assert_eq!(
            serde_json::to_value(Headers(non_ascii())).unwrap(),
            json!({
                "content-type": "text/plain",
                "set-cookie": ["a=1", "b=2", [99, 61, 233]],
                "x-name": [[99, 97, 102, 233]],
            }),
        );
    }

    #[test]
    fn text_formats_read_what_http_serde_wrote() {
        let written = serde_json::to_string(&HttpSerdeHeaders(ascii())).unwrap();
        let Headers(read) = serde_json::from_str(&written).unwrap();
        assert_eq!(read, ascii());
    }

    #[test]
    fn text_formats_keep_every_value() {
        let json = serde_json::to_string(&Headers(non_ascii())).unwrap();
        let Headers(read) = serde_json::from_str(&json).unwrap();
        assert_eq!(read, non_ascii());

        let ron = ron::to_string(&Headers(non_ascii())).unwrap();
        let Headers(read) = ron::from_str(&ron).unwrap();
        assert_eq!(read, non_ascii());
    }

    #[test]
    fn binary_formats_write_like_http_serde() {
        assert_eq!(
            encode_to_vec(Headers(non_ascii()), standard()).unwrap(),
            encode_to_vec(HttpSerdeHeaders(non_ascii()), standard()).unwrap(),
        );
    }
}
