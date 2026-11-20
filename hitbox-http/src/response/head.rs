//! The response head (status, version, headers and trailers) as one byte string.
//!
//! Binary formats store a [`SerializableHttpResponse`](super::SerializableHttpResponse) as two
//! byte strings: this head, then the body. Headers never pass through the outer format, so their
//! cost doesn't depend on the format or on erased-serde.
//!
//! The head is a postcard encoding of [`HeadOut`]: a layout version byte, the status, the HTTP
//! version, then headers and trailers as sequences of (name, value) byte strings. A name with
//! several values appears once for each value.

use std::fmt;

use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version};
use postcard::{take_from_bytes, to_extend};
use serde::de::{Deserializer, Error, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize, Serializer};
use serde_bytes::Bytes as ByteSlice;

/// The layout written by [`encode`]. Change it whenever [`HeadOut`] changes.
const LAYOUT_VERSION: u8 = 1;

/// A decoded head.
pub(super) struct Head {
    pub(super) status: StatusCode,
    pub(super) version: Version,
    pub(super) headers: HeaderMap,
    pub(super) trailers: HeaderMap,
}

/// Why a head can't be decoded.
#[derive(Debug)]
pub(super) enum HeadError {
    UnsupportedLayout(u8),
    Invalid(&'static str),
    // postcard drops the message of custom errors, so an invalid header name or value shows up
    // here as a generic deserialization error.
    Postcard(postcard::Error),
}

impl fmt::Display for HeadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedLayout(layout) => {
                write!(f, "unsupported response head layout version {layout}")
            }
            Self::Invalid(reason) => write!(f, "invalid response head: {reason}"),
            Self::Postcard(err) => write!(f, "invalid response head: {err}"),
        }
    }
}

const fn version_id(version: Version) -> u8 {
    match version {
        Version::HTTP_09 => 0,
        Version::HTTP_10 => 1,
        Version::HTTP_2 => 3,
        Version::HTTP_3 => 4,
        // `Version` has no other public values.
        _ => 2,
    }
}

fn version_from_id(id: u8) -> Result<Version, HeadError> {
    match id {
        0 => Ok(Version::HTTP_09),
        1 => Ok(Version::HTTP_10),
        2 => Ok(Version::HTTP_11),
        3 => Ok(Version::HTTP_2),
        4 => Ok(Version::HTTP_3),
        _ => Err(HeadError::Invalid("unknown HTTP version")),
    }
}

#[derive(Serialize)]
struct HeadOut<'a> {
    layout: u8,
    status: u16,
    version: u8,
    headers: PairsOut<'a>,
    trailers: PairsOut<'a>,
}

struct PairsOut<'a>(&'a HeaderMap);

impl Serialize for PairsOut<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // postcard needs the length up front, and `HeaderMap::iter` doesn't report it exactly.
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for (name, value) in self.0 {
            seq.serialize_element(&(
                ByteSlice::new(name.as_str().as_bytes()),
                ByteSlice::new(value.as_bytes()),
            ))?;
        }
        seq.end()
    }
}

#[derive(Deserialize)]
struct HeadIn {
    layout: u8,
    status: u16,
    version: u8,
    headers: PairsIn,
    trailers: PairsIn,
}

struct PairsIn(HeaderMap);

impl<'de> Deserialize<'de> for PairsIn {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(PairsVisitor)
    }
}

struct PairsVisitor;

impl<'de> Visitor<'de> for PairsVisitor {
    type Value = PairsIn;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence of header names and values")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<PairsIn, A::Error> {
        // The count comes from the input, so preallocate only a little.
        let mut map = HeaderMap::try_with_capacity(seq.size_hint().unwrap_or(0).min(32))
            .map_err(A::Error::custom)?;
        // Names are read as bytes: `HeaderName` validates them, so a UTF-8 check would be extra.
        while let Some((name, value)) = seq.next_element::<(&ByteSlice, &ByteSlice)>()? {
            let name = HeaderName::from_bytes(name).map_err(A::Error::custom)?;
            let value = HeaderValue::from_bytes(value).map_err(A::Error::custom)?;
            map.try_append(name, value).map_err(A::Error::custom)?;
        }
        Ok(PairsIn(map))
    }
}

/// Bytes a map needs when every length fits in two bytes.
fn map_size(map: &HeaderMap) -> usize {
    map.iter()
        .map(|(name, value)| name.as_str().len() + value.len() + 4)
        .sum::<usize>()
        + 2
}

/// Encodes the head.
pub(super) fn encode(
    status: StatusCode,
    version: Version,
    headers: &HeaderMap,
    trailers: &HeaderMap,
) -> Vec<u8> {
    let head = HeadOut {
        layout: LAYOUT_VERSION,
        status: status.as_u16(),
        version: version_id(version),
        headers: PairsOut(headers),
        trailers: PairsOut(trailers),
    };
    let buf = Vec::with_capacity(6 + map_size(headers) + map_size(trailers));
    // Writing to a `Vec` can't fail, and postcard supports every field of `HeadOut`.
    to_extend(&head, buf).expect("postcard writes a head to a Vec")
}

/// Decodes a head written by [`encode`].
pub(super) fn decode(bytes: &[u8]) -> Result<Head, HeadError> {
    // postcard writes a `u8` as one byte, so the layout is the first byte. Checking it first
    // reports a future layout as such instead of as broken input.
    match bytes.first() {
        Some(&LAYOUT_VERSION) => {}
        Some(&other) => return Err(HeadError::UnsupportedLayout(other)),
        None => return Err(HeadError::Invalid("empty")),
    }
    let (head, rest) = take_from_bytes::<HeadIn>(bytes).map_err(HeadError::Postcard)?;
    if !rest.is_empty() {
        return Err(HeadError::Invalid("trailing bytes"));
    }
    debug_assert_eq!(head.layout, LAYOUT_VERSION);
    Ok(Head {
        status: StatusCode::from_u16(head.status)
            .map_err(|_| HeadError::Invalid("invalid status"))?,
        version: version_from_id(head.version)?,
        headers: head.headers.0,
        trailers: head.trailers.0,
    })
}

/// An encoded head, written as a byte string.
pub(super) struct HeadBytes<'a>(pub(super) &'a [u8]);

impl Serialize for HeadBytes<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.0)
    }
}

impl<'de> Deserialize<'de> for Head {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_bytes(HeadVisitor)
    }
}

struct HeadVisitor;

impl<'de> Visitor<'de> for HeadVisitor {
    type Value = Head;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an encoded response head")
    }

    fn visit_bytes<E: Error>(self, bytes: &[u8]) -> Result<Head, E> {
        decode(bytes).map_err(E::custom)
    }

    // For formats that write byte strings as sequences.
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Head, A::Error> {
        let mut bytes = Vec::new();
        while let Some(byte) = seq.next_element::<u8>()? {
            bytes.push(byte);
        }
        decode(&bytes).map_err(A::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use bincode::config::standard;
    use bincode::serde::{decode_from_slice, encode_to_vec};
    use bytes::Bytes;
    use http::header::{CONTENT_TYPE, SET_COOKIE};

    use super::*;
    use crate::SerializableHttpResponse;

    fn headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.append(SET_COOKIE, HeaderValue::from_static("a=1"));
        headers.append(SET_COOKIE, HeaderValue::from_static("b=2"));
        headers.insert(
            "x-binary",
            HeaderValue::from_bytes(&[b'a', 0xFF, 0x80]).unwrap(),
        );
        headers.insert("x-long", HeaderValue::from_bytes(&[b'x'; 300]).unwrap());
        headers
    }

    fn trailers() -> HeaderMap {
        let mut trailers = HeaderMap::new();
        trailers.insert("grpc-status", HeaderValue::from_static("0"));
        trailers
    }

    #[test]
    fn round_trip_keeps_everything() {
        for version in [
            Version::HTTP_09,
            Version::HTTP_10,
            Version::HTTP_11,
            Version::HTTP_2,
            Version::HTTP_3,
        ] {
            let bytes = encode(StatusCode::CREATED, version, &headers(), &trailers());
            let head = decode(&bytes).unwrap();
            assert_eq!(head.status, StatusCode::CREATED);
            assert_eq!(head.version, version);
            assert_eq!(head.headers, headers());
            assert_eq!(head.trailers, trailers());
        }
    }

    #[test]
    fn empty_maps_round_trip() {
        let empty = HeaderMap::new();
        let head = decode(&encode(StatusCode::OK, Version::HTTP_11, &empty, &empty)).unwrap();
        assert!(head.headers.is_empty());
        assert!(head.trailers.is_empty());
    }

    #[test]
    fn layout_is_stable() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        let bytes = encode(StatusCode::OK, Version::HTTP_11, &headers, &trailers());
        // Layout 1, status 200 as a varint, HTTP/1.1, one header, one trailer.
        let expected = [
            &[LAYOUT_VERSION, 0xC8, 0x01, 2, 1, 12][..],
            b"content-type",
            &[10],
            b"text/plain",
            &[1, 11],
            b"grpc-status",
            &[1],
            b"0",
        ]
        .concat();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn unknown_layout_version_is_an_error() {
        let mut bytes = encode(StatusCode::OK, Version::HTTP_11, &headers(), &trailers());
        bytes[0] = LAYOUT_VERSION + 1;
        let err = decode(&bytes).err().unwrap();
        assert_eq!(
            err.to_string(),
            format!(
                "unsupported response head layout version {}",
                LAYOUT_VERSION + 1
            )
        );
    }

    #[test]
    fn truncated_input_is_an_error() {
        let bytes = encode(StatusCode::OK, Version::HTTP_11, &headers(), &trailers());
        for len in 0..bytes.len() {
            assert!(decode(&bytes[..len]).is_err(), "prefix of {len} bytes");
        }
    }

    #[test]
    fn trailing_bytes_are_an_error() {
        let mut bytes = encode(StatusCode::OK, Version::HTTP_11, &headers(), &trailers());
        bytes.push(0);
        assert!(decode(&bytes).is_err());
    }

    #[test]
    fn wrong_input_is_an_error_not_a_panic() {
        // A header count far above what the input can hold.
        let huge_count = [LAYOUT_VERSION, 0xC8, 0x01, 2, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
        assert!(decode(&huge_count).is_err());
        // Every byte value at every position of an otherwise valid head.
        let valid = encode(StatusCode::OK, Version::HTTP_11, &headers(), &trailers());
        for position in 0..valid.len() {
            for byte in 0..=u8::MAX {
                let mut bytes = valid.clone();
                bytes[position] = byte;
                let _ = decode(&bytes);
            }
        }
    }

    #[test]
    fn binary_formats_get_a_head_and_a_body() {
        let head = encode(StatusCode::OK, Version::HTTP_2, &headers(), &trailers());
        let body = Bytes::from_static(b"body");
        let written = encode_to_vec((HeadBytes(&head), &body), standard()).unwrap();

        let (response, _): (SerializableHttpResponse, _) =
            decode_from_slice(&written, standard()).unwrap();
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.version, Version::HTTP_2);
        assert_eq!(response.body, body);
        assert_eq!(response.headers, headers());
        assert_eq!(response.trailers, trailers());
        assert_eq!(encode_to_vec(&response, standard()).unwrap(), written);
    }

    #[test]
    fn a_bad_head_is_a_deserialize_error() {
        let written = encode_to_vec(
            (HeadBytes(&[LAYOUT_VERSION, 0xC8]), Bytes::new()),
            standard(),
        )
        .unwrap();
        let err = decode_from_slice::<SerializableHttpResponse, _>(&written, standard())
            .err()
            .unwrap();
        assert!(err.to_string().contains("invalid response head"), "{err}");
    }
}
