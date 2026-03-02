//! gRPC frame handling.
//!
//! gRPC messages are wrapped in a 5-byte Length-Prefixed Message frame:
//! - 1 byte: compressed flag (0 = not compressed, 1 = compressed)
//! - 4 bytes: big-endian message length
//! - N bytes: protobuf message
//!
//! This module provides functions to strip and add this framing,
//! plus a [`BytesExtractor`](hitbox_proto::decode::BytesExtractor) for use
//! with `hitbox-proto` predicates and extractors.

use bytes::{BufMut, Bytes, BytesMut};

/// gRPC frame header size in bytes (1 byte flag + 4 bytes length).
pub const GRPC_FRAME_HEADER_SIZE: usize = 5;

/// Strips the gRPC 5-byte frame header, returning the protobuf payload.
///
/// Returns `None` if:
/// - The input is shorter than 5 bytes
/// - The declared length doesn't match the remaining bytes
///
/// # Wire Format
///
/// ```text
/// [compressed: u8][length: u32 BE][payload: bytes]
/// ```
pub fn strip_grpc_frame(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < GRPC_FRAME_HEADER_SIZE {
        return None;
    }

    let length = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
    let payload = &bytes[GRPC_FRAME_HEADER_SIZE..];

    if payload.len() != length {
        return None;
    }

    Some(payload)
}

/// Wraps a protobuf payload in a gRPC frame.
///
/// # Arguments
///
/// * `payload` - The protobuf message bytes
/// * `compressed` - Whether the payload is compressed
pub fn add_grpc_frame(payload: &[u8], compressed: bool) -> Bytes {
    let mut buf = BytesMut::with_capacity(GRPC_FRAME_HEADER_SIZE + payload.len());
    buf.put_u8(if compressed { 1 } else { 0 });
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

/// A [`BytesExtractor`](hitbox_proto::decode::BytesExtractor) that strips gRPC framing.
///
/// Use this when configuring `hitbox-proto` predicates and extractors for gRPC:
///
/// ```ignore
/// use hitbox_grpc::frame::grpc_bytes_extractor;
/// use hitbox_proto::predicates::field::ProtoField;
///
/// let predicate = ProtoField::new(msg_desc, "user_id", op, grpc_bytes_extractor);
/// ```
pub fn grpc_bytes_extractor(bytes: &[u8]) -> Option<&[u8]> {
    strip_grpc_frame(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_valid_frame() {
        let payload = b"hello";
        let framed = add_grpc_frame(payload, false);
        let stripped = strip_grpc_frame(&framed).unwrap();
        assert_eq!(stripped, payload);
    }

    #[test]
    fn strip_compressed_frame() {
        let payload = b"compressed_data";
        let framed = add_grpc_frame(payload, true);
        assert_eq!(framed[0], 1); // compressed flag
        let stripped = strip_grpc_frame(&framed).unwrap();
        assert_eq!(stripped, payload);
    }

    #[test]
    fn strip_empty_payload() {
        let framed = add_grpc_frame(&[], false);
        assert_eq!(framed.len(), GRPC_FRAME_HEADER_SIZE);
        let stripped = strip_grpc_frame(&framed).unwrap();
        assert!(stripped.is_empty());
    }

    #[test]
    fn strip_truncated_header() {
        assert!(strip_grpc_frame(&[0, 0, 0]).is_none());
        assert!(strip_grpc_frame(&[]).is_none());
    }

    #[test]
    fn strip_length_mismatch() {
        // Header says 10 bytes but only 5 follow
        let mut buf = vec![0u8]; // not compressed
        buf.extend_from_slice(&10u32.to_be_bytes());
        buf.extend_from_slice(b"hello"); // only 5 bytes
        assert!(strip_grpc_frame(&buf).is_none());
    }

    #[test]
    fn round_trip() {
        let original = b"test protobuf message bytes";
        let framed = add_grpc_frame(original, false);
        let recovered = strip_grpc_frame(&framed).unwrap();
        assert_eq!(recovered, original);
    }

    #[test]
    fn frame_header_format() {
        let payload = vec![1, 2, 3, 4];
        let framed = add_grpc_frame(&payload, false);

        assert_eq!(framed[0], 0); // not compressed
        assert_eq!(&framed[1..5], &[0, 0, 0, 4]); // length = 4
        assert_eq!(&framed[5..], &[1, 2, 3, 4]); // payload
    }

    #[test]
    fn grpc_bytes_extractor_works() {
        let payload = b"proto data";
        let framed = add_grpc_frame(payload, false);
        let extracted = grpc_bytes_extractor(&framed).unwrap();
        assert_eq!(extracted, payload);
    }
}
