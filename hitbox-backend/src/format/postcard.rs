use ::postcard::Serializer;
use ::postcard::ser_flavors::{AllocVec, Flavor};
use bytes::Bytes;
use hitbox_core::{BoxContext, Raw};
use thiserror::Error;

use super::{
    Format, FormatDeserializer, FormatError, FormatSerializer, FormatTypeId, PostcardDecoder,
    PostcardEncoder,
};
use crate::context::Context;

/// Fast, compact binary serialization with a stable wire format.
///
/// Uses [postcard](https://docs.rs/postcard), whose wire format is stable since 1.0.
/// Not human-readable and not self-describing: the bytes carry no field names or
/// types, so serde features that need them fail at runtime. Types that use
/// `#[serde(untagged)]`, `#[serde(skip_serializing_if)]` or `serde_json::Value`
/// fail to decode, and types that use `#[serde(flatten)]` fail to encode. Use
/// [`JsonFormat`](super::JsonFormat) for such types.
///
/// Decoding rejects input with bytes left after the value.
#[derive(Debug, Clone, Copy, Default)]
pub struct PostcardFormat;

/// Input that holds more than one value.
#[derive(Debug, Error)]
#[error("{0} bytes left after the decoded value")]
pub(super) struct TrailingBytes(pub(super) usize);

impl Format for PostcardFormat {
    fn with_serializer(
        &self,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        _context: &dyn Context,
    ) -> Result<Raw, FormatError> {
        let mut serializer = Serializer {
            output: AllocVec::new(),
        };
        let mut format_ser = FormatSerializer::Postcard(PostcardEncoder(&mut serializer));
        f(&mut format_ser)?;

        // Drop the growth slack: in-process backends store the buffer as is
        let mut buf = serializer
            .output
            .finalize()
            .map_err(|e| FormatError::Serialize(Box::new(e)))?;
        buf.shrink_to_fit();
        Ok(Bytes::from(buf))
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut FormatDeserializer) -> Result<(), FormatError>,
        _ctx: &mut BoxContext,
    ) -> Result<(), FormatError> {
        f(&mut FormatDeserializer::Postcard(PostcardDecoder(data)))
    }

    fn clone_box(&self) -> Box<dyn Format> {
        Box::new(*self)
    }

    fn format_type_id(&self) -> FormatTypeId {
        FormatTypeId::Postcard
    }
}

#[cfg(test)]
mod tests {
    use ::postcard::to_allocvec;
    use hitbox_core::CacheContext;
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::format::FormatExt;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[cfg_attr(
        feature = "rkyv_format",
        derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
    )]
    struct Value {
        id: u64,
        name: String,
        tags: Vec<String>,
        limit: Option<u32>,
        empty: Vec<u8>,
    }

    fn value() -> Value {
        Value {
            id: u64::MAX,
            name: "x".repeat(300),
            tags: vec!["a".to_owned(), "b".to_owned()],
            limit: Some(300),
            empty: Vec::new(),
        }
    }

    fn encode(value: &Value) -> Raw {
        PostcardFormat
            .serialize(value, &CacheContext::default())
            .unwrap()
    }

    fn decode(raw: &Raw) -> Result<Value, FormatError> {
        PostcardFormat.deserialize(raw, &mut CacheContext::default().boxed())
    }

    #[test]
    fn round_trip_keeps_the_value() {
        assert_eq!(decode(&encode(&value())).unwrap(), value());
    }

    #[test]
    fn writes_the_postcard_wire_format() {
        assert_eq!(encode(&value()), to_allocvec(&value()).unwrap());
    }

    #[test]
    fn trailing_bytes_are_an_error() {
        let mut bytes = encode(&value()).to_vec();
        bytes.push(0);
        let err = decode(&Raw::from(bytes)).unwrap_err();
        assert_eq!(err.to_string(), "1 bytes left after the decoded value");
    }

    #[test]
    fn truncated_input_is_an_error() {
        let bytes = encode(&value());
        for len in 0..bytes.len() {
            assert!(
                decode(&bytes.slice(..len)).is_err(),
                "prefix of {len} bytes"
            );
        }
    }
}
