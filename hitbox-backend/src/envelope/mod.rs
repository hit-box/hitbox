//! Unified, zero-copy value envelope for backends without native metadata.
//!
//! Some backends cannot natively store the per-entry `expire`/`stale`
//! timestamps that Hitbox attaches to every cache value. Object storage (S3)
//! and embedded stores without sub-second per-key TTL (FeOxDB, redb) fall into
//! this category: the metadata has to travel *inside* the stored blob.
//!
//! [`ValueEnvelope`] packs `expire`/`stale` into a fixed little-endian header
//! followed by the raw, already-serialized/compressed value bytes. The payload
//! is never re-serialized — serialization appends it after the header, and
//! deserialization slices it back out. This avoids the double-serialization
//! penalty of wrapping value bytes in a second serde/bincode pass.
//!
//! # Binary layout
//!
//! ```text
//! offset  size  field
//! 0       1     version        u8   (= 1)
//! 1       1     flags          u8   (bit0 = expire present, bit1 = stale present)
//! 2       2     reserved       u16  (= 0)
//! 4       8     expire_secs    i64 LE   (Unix seconds; valid iff flags bit0)
//! 12      4     expire_nanos   u32 LE   (sub-second nanos)
//! 16      8     stale_secs     i64 LE   (Unix seconds; valid iff flags bit1)
//! 24      4     stale_nanos    u32 LE   (sub-second nanos)
//! 28      ..    payload        raw value bytes (length = total_len - 28)
//! ```
//!
//! # Forward compatibility
//!
//! The leading version byte lets the format evolve without a hard break: a
//! future reader dispatches on the version. Multi-byte integers are encoded
//! explicitly little-endian (never native-endian casting) so a blob written on
//! one machine decodes identically on another.

mod codec;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use tracing::warn;

use hitbox_core::{CacheValue, Raw};

use crate::{BackendError, BackendResult};

use codec::{
    decode_timestamp, encode_timestamp, read_i64_le, read_u32_le, write_i64_le, write_u32_le,
};

/// Current envelope format version. Stored as the first byte of every blob.
pub const ENVELOPE_VERSION: u8 = 1;

/// Size of the fixed header in bytes. The payload begins at this offset.
pub const HEADER_SIZE: usize = 28;

const FLAG_EXPIRE_PRESENT: u8 = 0b0000_0001;
const FLAG_STALE_PRESENT: u8 = 0b0000_0010;

/// A reusable, zero-copy envelope for backends that lack native TTL/metadata
/// storage (e.g. S3, FeOxDB, redb).
///
/// Stores `expire`/`stale` timestamps in a fixed little-endian header followed
/// by the raw value bytes. See the [module docs](self) for the binary layout
/// and design rationale.
///
/// # Example
///
/// ```
/// use bytes::Bytes;
/// use chrono::Utc;
/// use hitbox_backend::ValueEnvelope;
/// use hitbox_core::CacheValue;
///
/// let value = CacheValue::new(
///     Bytes::from_static(b"cached bytes"),
///     Some(Utc::now() + chrono::Duration::seconds(60)),
///     None,
/// );
///
/// let bytes = ValueEnvelope::from(value).serialize().unwrap();
/// let envelope = ValueEnvelope::deserialize(bytes).unwrap();
/// assert_eq!(&envelope.data[..], b"cached bytes");
/// ```
#[derive(Debug, Clone)]
pub struct ValueEnvelope {
    /// Raw, already-serialized/compressed value bytes.
    pub data: Bytes,
    /// When the entry expires (becomes invalid). `None` means no expiration.
    pub expire: Option<DateTime<Utc>>,
    /// When the entry becomes stale (should be revalidated). `None` means never.
    pub stale: Option<DateTime<Utc>>,
}

impl ValueEnvelope {
    /// Serializes the header and raw payload into a single buffer.
    ///
    /// The payload bytes are appended verbatim after the 28-byte header; they
    /// are never re-encoded.
    pub fn serialize(&self) -> BackendResult<Bytes> {
        let (expire_secs, expire_nanos) = encode_timestamp(self.expire);
        let (stale_secs, stale_nanos) = encode_timestamp(self.stale);

        let mut flags = 0u8;
        if self.expire.is_some() {
            flags |= FLAG_EXPIRE_PRESENT;
        }
        if self.stale.is_some() {
            flags |= FLAG_STALE_PRESENT;
        }

        let mut buf = Vec::with_capacity(HEADER_SIZE + self.data.len());
        buf.push(ENVELOPE_VERSION); // offset 0
        buf.push(flags); // offset 1
        buf.extend_from_slice(&0u16.to_le_bytes()); // offset 2: reserved
        write_i64_le(&mut buf, expire_secs); // offset 4
        write_u32_le(&mut buf, expire_nanos); // offset 12
        write_i64_le(&mut buf, stale_secs); // offset 16
        write_u32_le(&mut buf, stale_nanos); // offset 24

        debug_assert_eq!(buf.len(), HEADER_SIZE);

        buf.extend_from_slice(&self.data); // offset 28: payload

        Ok(Bytes::from(buf))
    }

    /// Parses a buffer produced by [`serialize`](Self::serialize).
    ///
    /// Accepts anything convertible into [`Bytes`] (e.g. an owned `Bytes` or
    /// `Vec<u8>`); the payload is returned as a **zero-copy slice** of that
    /// buffer rather than copied out.
    ///
    /// Returns an error if the buffer is shorter than [`HEADER_SIZE`], if the
    /// version byte is unknown, or if the encoded timestamps are out of range.
    /// Callers that want graceful degradation across a format migration should
    /// use [`decode_unexpired`](Self::decode_unexpired), which maps a decode
    /// error to a cache miss.
    pub fn deserialize(data: impl Into<Bytes>) -> BackendResult<Self> {
        let data = data.into();
        if data.len() < HEADER_SIZE {
            return Err(too_short(data.len()));
        }

        let version = data[0];
        if version != ENVELOPE_VERSION {
            return Err(unknown_version(version));
        }

        let flags = data[1];
        // bytes [2..4] are reserved and ignored on read.

        let expire_present = flags & FLAG_EXPIRE_PRESENT != 0;
        let stale_present = flags & FLAG_STALE_PRESENT != 0;

        let expire_secs = read_i64_le(&data[4..12]);
        let expire_nanos = read_u32_le(&data[12..16]);
        let stale_secs = read_i64_le(&data[16..24]);
        let stale_nanos = read_u32_le(&data[24..28]);

        let expire =
            decode_optional_timestamp(expire_present, expire_secs, expire_nanos, "expire")?;
        let stale = decode_optional_timestamp(stale_present, stale_secs, stale_nanos, "stale")?;

        // Zero-copy: the payload shares the input buffer instead of being copied.
        let payload = data.slice(HEADER_SIZE..);

        Ok(Self {
            data: payload,
            expire,
            stale,
        })
    }

    /// Decodes an envelope and applies the read-time policy shared by every
    /// backend without native TTL/metadata (S3, FeOxDB, …):
    ///
    /// - a decode failure is treated as a **cache miss** — logged at `warn` and
    ///   counted via the `decode_errors` metric, never propagated — so a format
    ///   migration or on-disk corruption degrades gracefully;
    /// - an entry whose `expire` is in the past is also a miss, using
    ///   `expire <= now` to match `CacheValue::cache_state`.
    ///
    /// Returns the live value otherwise. Centralizing this keeps the envelope
    /// backends from drifting in how they treat decode errors and expiry.
    /// `backend` is the backend label used for the decode-error metric.
    pub fn decode_unexpired(data: impl Into<Bytes>, backend: &str) -> Option<CacheValue<Raw>> {
        let envelope = match Self::deserialize(data) {
            Ok(envelope) => envelope,
            Err(error) => {
                warn!(%error, backend, "failed to decode value envelope; treating as cache miss");
                crate::metrics::record_decode_error(backend);
                return None;
            }
        };

        let value: CacheValue<Raw> = envelope.into();
        if let Some(expire) = value.expire()
            && expire <= Utc::now()
        {
            return None;
        }
        Some(value)
    }
}

impl From<CacheValue<Raw>> for ValueEnvelope {
    fn from(v: CacheValue<Raw>) -> Self {
        Self {
            // `Bytes::clone` is a cheap refcount bump, not a deep copy.
            data: v.data().clone(),
            expire: v.expire(),
            stale: v.stale(),
        }
    }
}

impl From<ValueEnvelope> for CacheValue<Raw> {
    fn from(e: ValueEnvelope) -> Self {
        CacheValue::new(e.data, e.expire, e.stale)
    }
}

/// Decodes one optional timestamp field: `None` when absent, or an error when
/// the stored `(secs, nanos)` pair is out of range.
fn decode_optional_timestamp(
    present: bool,
    secs: i64,
    nanos: u32,
    field: &str,
) -> BackendResult<Option<DateTime<Utc>>> {
    if !present {
        return Ok(None);
    }
    decode_timestamp(secs, nanos)
        .map(Some)
        .ok_or_else(|| invalid_timestamp(field, secs, nanos))
}

fn too_short(len: usize) -> BackendError {
    BackendError::InternalError(Box::new(std::io::Error::new(
        std::io::ErrorKind::UnexpectedEof,
        format!("value envelope too short: expected >= {HEADER_SIZE} bytes, got {len}"),
    )))
}

fn unknown_version(version: u8) -> BackendError {
    BackendError::InternalError(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("unknown value envelope version: {version} (expected {ENVELOPE_VERSION})"),
    )))
}

fn invalid_timestamp(field: &str, secs: i64, nanos: u32) -> BackendError {
    BackendError::InternalError(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("invalid {field} timestamp in value envelope: secs={secs}, nanos={nanos}"),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample(data: &[u8], expire: Option<DateTime<Utc>>, stale: Option<DateTime<Utc>>) -> Bytes {
        ValueEnvelope {
            data: Bytes::copy_from_slice(data),
            expire,
            stale,
        }
        .serialize()
        .unwrap()
    }

    #[test]
    fn header_size_is_28() {
        assert_eq!(HEADER_SIZE, 28);
        let bytes = sample(b"x", None, None);
        assert_eq!(bytes.len(), HEADER_SIZE + 1);
    }

    #[test]
    fn roundtrip_preserves_all_fields() {
        let expire = Some(DateTime::from_timestamp(1_700_000_000, 123_456_789).unwrap());
        let stale = Some(DateTime::from_timestamp(1_699_999_000, 987_654_321).unwrap());
        let data = vec![1u8, 2, 3, 4, 5];

        let bytes = sample(&data, expire, stale);
        let envelope = ValueEnvelope::deserialize(bytes).unwrap();

        assert_eq!(&envelope.data[..], &data[..]);
        assert_eq!(envelope.expire, expire);
        assert_eq!(envelope.stale, stale);
        // Sub-second precision survives.
        assert_eq!(
            envelope.expire.unwrap().timestamp_subsec_nanos(),
            123_456_789
        );
        assert_eq!(
            envelope.stale.unwrap().timestamp_subsec_nanos(),
            987_654_321
        );
    }

    #[test]
    fn epoch_expire_roundtrips_as_some() {
        // Guards the flags-byte design: an entry expiring exactly at the Unix
        // epoch must survive as Some(epoch), not collapse to None.
        let epoch = DateTime::from_timestamp(0, 0).unwrap();
        let bytes = sample(b"x", Some(epoch), None);
        let env = ValueEnvelope::deserialize(bytes).unwrap();
        assert_eq!(env.expire, Some(epoch));
        assert_eq!(env.stale, None);
    }

    #[test]
    fn roundtrip_all_flag_combinations() {
        let ts = Utc::now();
        let combos = [
            (None, None),
            (Some(ts), None),
            (None, Some(ts)),
            (Some(ts), Some(ts + Duration::seconds(30))),
        ];
        for (expire, stale) in combos {
            let bytes = sample(b"payload", expire, stale);
            let env = ValueEnvelope::deserialize(bytes).unwrap();
            assert_eq!(env.expire, expire);
            assert_eq!(env.stale, stale);
            assert_eq!(&env.data[..], b"payload");
        }
    }

    #[test]
    fn empty_payload_roundtrips() {
        let bytes = sample(b"", Some(Utc::now()), None);
        let env = ValueEnvelope::deserialize(bytes).unwrap();
        assert!(env.data.is_empty());
        assert!(env.expire.is_some());
    }

    #[test]
    fn large_payload_roundtrips() {
        let data = vec![0xABu8; 1024 * 1024];
        let bytes = sample(&data, None, None);
        let env = ValueEnvelope::deserialize(bytes).unwrap();
        assert_eq!(env.data.len(), data.len());
        assert_eq!(&env.data[..], &data[..]);
    }

    #[test]
    fn payload_is_not_reencoded() {
        // The bytes after the header must be byte-identical to the input.
        let data = vec![9u8, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        let bytes = sample(&data, Some(Utc::now()), Some(Utc::now()));
        assert_eq!(&bytes[HEADER_SIZE..], &data[..]);
    }

    #[test]
    fn deserialize_too_short_errors() {
        let short = vec![0u8; HEADER_SIZE - 1];
        assert!(ValueEnvelope::deserialize(short).is_err());
    }

    #[test]
    fn deserialize_unknown_version_errors() {
        let mut bytes = sample(b"data", None, None).to_vec();
        bytes[0] = 99; // bogus version
        assert!(ValueEnvelope::deserialize(bytes).is_err());
    }

    #[test]
    fn cache_value_conversion_roundtrip() {
        let expire = Some(Utc::now() + Duration::seconds(60));
        let stale = Some(Utc::now() + Duration::seconds(30));
        let value = CacheValue::new(Bytes::from_static(b"abc"), expire, stale);

        let bytes = ValueEnvelope::from(value).serialize().unwrap();
        let restored: CacheValue<Raw> = ValueEnvelope::deserialize(bytes).unwrap().into();

        assert_eq!(&restored.data()[..], b"abc");
        assert_eq!(restored.expire(), expire);
        assert_eq!(restored.stale(), stale);
    }

    #[test]
    fn handcrafted_little_endian_header_decodes() {
        // Build a header by hand with known little-endian bytes and assert it
        // decodes to the expected timestamp. Guards against native-endian
        // regressions.
        let mut bytes = Vec::new();
        bytes.push(ENVELOPE_VERSION); // version
        bytes.push(FLAG_EXPIRE_PRESENT); // flags: expire present, stale absent
        bytes.extend_from_slice(&0u16.to_le_bytes()); // reserved
        // expire_secs = 1 (little-endian)
        bytes.extend_from_slice(&1i64.to_le_bytes());
        // expire_nanos = 0
        bytes.extend_from_slice(&0u32.to_le_bytes());
        // stale_secs = 0
        bytes.extend_from_slice(&0i64.to_le_bytes());
        // stale_nanos = 0
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(b"hello");

        let env = ValueEnvelope::deserialize(bytes).unwrap();
        assert_eq!(env.expire, Some(DateTime::from_timestamp(1, 0).unwrap()));
        assert_eq!(env.stale, None);
        assert_eq!(&env.data[..], b"hello");
    }

    #[test]
    fn decode_unexpired_returns_value_when_fresh() {
        let bytes = sample(b"fresh", Some(Utc::now() + Duration::seconds(60)), None);
        let value = ValueEnvelope::decode_unexpired(bytes, "test").expect("present");
        assert_eq!(value.data().as_ref(), b"fresh");
    }

    #[test]
    fn decode_unexpired_treats_expired_as_miss() {
        let bytes = sample(b"old", Some(Utc::now() - Duration::seconds(1)), None);
        assert!(ValueEnvelope::decode_unexpired(bytes, "test").is_none());
    }

    #[test]
    fn decode_unexpired_treats_garbage_as_miss() {
        assert!(ValueEnvelope::decode_unexpired(b"not-an-envelope".to_vec(), "test").is_none());
    }
}
