//! Little-endian primitives and timestamp codec shared by envelope formats.
//!
//! These helpers encode multi-byte integers with explicit little-endian byte
//! order (never relying on native-endian casting), so that values produced on
//! one machine decode identically on another. This matters for any backend
//! whose bytes cross or move between machines (S3 objects, persistent files).
//!
//! The timestamp codec splits an `Option<DateTime<Utc>>` into a presence flag,
//! a seconds component and a sub-second nanoseconds component. A presence flag
//! is used rather than an all-zero sentinel because `0` is a valid Unix epoch
//! timestamp — treating it as "absent" would silently corrupt an entry that
//! expires exactly at the epoch.

use chrono::{DateTime, Utc};

/// Splits an optional timestamp into its `(seconds, nanos)` parts.
///
/// `None` maps to `(0, 0)`; presence is recorded separately by the caller (a
/// flags byte), so an epoch-0 timestamp is never mistaken for "absent".
pub(crate) fn encode_timestamp(timestamp: Option<DateTime<Utc>>) -> (i64, u32) {
    match timestamp {
        Some(dt) => (dt.timestamp(), dt.timestamp_subsec_nanos()),
        None => (0, 0),
    }
}

/// Reconstructs a timestamp from its `(seconds, nanos)` parts, or `None` if the
/// pair is out of range (e.g. corrupt data). Presence is decided by the caller
/// via a flags byte, not by an all-zero sentinel.
pub(crate) fn decode_timestamp(secs: i64, nanos: u32) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(secs, nanos)
}

/// Writes an `i64` to `buf` in little-endian order.
#[inline]
pub(crate) fn write_i64_le(buf: &mut Vec<u8>, value: i64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Writes a `u32` to `buf` in little-endian order.
#[inline]
pub(crate) fn write_u32_le(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Reads an `i64` from `bytes` (must be at least 8 bytes) in little-endian order.
#[inline]
pub(crate) fn read_i64_le(bytes: &[u8]) -> i64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    i64::from_le_bytes(arr)
}

/// Reads a `u32` from `bytes` (must be at least 4 bytes) in little-endian order.
#[inline]
pub(crate) fn read_u32_le(bytes: &[u8]) -> u32 {
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[..4]);
    u32::from_le_bytes(arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn timestamp_roundtrip() {
        let ts = Utc::now();
        let (secs, nanos) = encode_timestamp(Some(ts));
        assert_eq!(decode_timestamp(secs, nanos), Some(ts));
    }

    #[test]
    fn none_encodes_to_zero() {
        assert_eq!(encode_timestamp(None), (0, 0));
    }

    #[test]
    fn epoch_roundtrips() {
        // Unix epoch encodes to (0, 0) and decodes back to the epoch (not None).
        // Absence is recorded by the envelope's flags byte, not an all-zero
        // sentinel, so an entry expiring at the epoch is never lost.
        let epoch = DateTime::from_timestamp(0, 0).unwrap();
        assert_eq!(encode_timestamp(Some(epoch)), (0, 0));
        assert_eq!(decode_timestamp(0, 0), Some(epoch));
    }

    #[test]
    fn subsecond_precision_preserved() {
        let ts = DateTime::from_timestamp(1_700_000_000, 123_456_789).unwrap();
        let (secs, nanos) = encode_timestamp(Some(ts));
        assert_eq!(decode_timestamp(secs, nanos), Some(ts));
    }

    #[test]
    fn integer_le_roundtrip() {
        let mut buf = Vec::new();
        write_i64_le(&mut buf, -42);
        write_u32_le(&mut buf, 4_000_000_000);
        assert_eq!(read_i64_le(&buf[0..8]), -42);
        assert_eq!(read_u32_le(&buf[8..12]), 4_000_000_000);
    }

    #[test]
    fn i64_is_little_endian() {
        let mut buf = Vec::new();
        write_i64_le(&mut buf, 1);
        // Little-endian: least significant byte first.
        assert_eq!(buf, vec![1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn future_timestamp_roundtrip() {
        let ts = Utc::now() + Duration::days(365 * 100);
        let (secs, nanos) = encode_timestamp(Some(ts));
        assert_eq!(decode_timestamp(secs, nanos), Some(ts));
    }
}
