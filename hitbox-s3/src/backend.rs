//! [`S3Backend`] — a cache backend backed by S3-compatible object storage.

use aws_sdk_s3::Client;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::primitives::ByteStream;
use hitbox_backend::{
    Backend, BackendError, BackendResult, CacheKeyFormat, Compressor, DeleteStatus, ValueEnvelope,
    format::Format,
};
use hitbox_core::{BackendLabel, CacheKey, CacheValue, Raw};

use crate::error::S3Error;

/// Maximum length of an S3 object key, in bytes (an S3 API limit).
const MAX_S3_KEY_LEN: usize = 1024;

/// Cache backend that stores each entry as a single object in S3-compatible
/// object storage.
///
/// Each cache value is stored as one object whose body is a
/// [`ValueEnvelope`]: a small header carrying `expire`/`stale` plus the raw
/// value bytes. S3 has no native per-key TTL, so expiration is enforced lazily
/// on read (an expired entry reads as a miss). Physical cleanup of expired
/// objects is the operator's responsibility via S3 Lifecycle rules.
///
/// Positioned as an **L3** tier: latency is ~50–200 ms on AWS Standard, so it
/// is best composed behind a fast L1 (Moka) and/or L2 (Redis). The concurrency
/// model is last-write-wins; there is no compare-and-swap.
///
/// Construct via [`S3Backend::builder`].
pub struct S3Backend {
    pub(crate) client: Client,
    pub(crate) bucket: String,
    pub(crate) prefix: String,
    pub(crate) label: BackendLabel,
    pub(crate) value_format: Box<dyn Format + Send + Sync>,
    pub(crate) key_format: CacheKeyFormat,
    pub(crate) compressor: Box<dyn Compressor + Send + Sync>,
}

impl std::fmt::Debug for S3Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Backend")
            .field("bucket", &self.bucket)
            .field("prefix", &self.prefix)
            .field("label", &self.label)
            .field("key_format", &self.key_format)
            .finish_non_exhaustive()
    }
}

impl Clone for S3Backend {
    fn clone(&self) -> Self {
        Self {
            // The client holds an `Arc` internally; cloning is cheap.
            client: self.client.clone(),
            bucket: self.bucket.clone(),
            prefix: self.prefix.clone(),
            label: self.label.clone(),
            value_format: self.value_format.clone_box(),
            key_format: self.key_format,
            compressor: self.compressor.clone_box(),
        }
    }
}

impl S3Backend {
    /// Starts building a new backend. `bucket` is required.
    pub fn builder(bucket: impl Into<String>) -> crate::builder::S3BackendBuilder {
        crate::builder::S3BackendBuilder::new(bucket)
    }

    /// Ensures the configured bucket exists, creating it if necessary.
    ///
    /// This is a convenience for tests and local development (e.g. against
    /// MinIO). It is **not** part of the production API: an application should
    /// not create its own cache bucket at runtime. Gated behind the
    /// `test-utils` feature.
    #[cfg(any(test, feature = "test-utils"))]
    pub async fn ensure_bucket(&self) -> Result<(), S3Error> {
        use aws_sdk_s3::operation::create_bucket::CreateBucketError;

        match self
            .client
            .create_bucket()
            .bucket(&self.bucket)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => match err.as_service_error() {
                // Already-exists is success for an idempotent ensure. Match the
                // typed variants — never string-match on the message, which
                // differs across MinIO/SDK versions.
                Some(
                    CreateBucketError::BucketAlreadyExists(_)
                    | CreateBucketError::BucketAlreadyOwnedByYou(_),
                ) => Ok(()),
                _ => Err(S3Error::from(err)),
            },
        }
    }
}

/// Builds the S3 object key for a cache key: `{prefix}/{hex(serialized_key)}`.
///
/// The serialized key bytes are hex-encoded so the result is always a valid,
/// collision-free S3 key regardless of `key_format`. Returns an error if the
/// resulting key would exceed the S3 key-length limit, rather than letting the
/// S3 API reject it with an opaque error.
pub(crate) fn encode_s3_key(
    prefix: &str,
    kf: &CacheKeyFormat,
    key: &CacheKey,
) -> BackendResult<String> {
    let bytes = kf
        .serialize(key)
        .map_err(|e| BackendError::InternalError(Box::new(e)))?;
    let hex = hex::encode(bytes);
    // Trim any trailing slash so a prefix passed as `"hitbox/"` does not
    // produce a double-slash key (`hitbox//<hex>`); the join below adds the
    // single separator.
    let prefix = prefix.trim_end_matches('/');
    let s3_key = if prefix.is_empty() {
        hex
    } else {
        format!("{prefix}/{hex}")
    };

    if s3_key.len() > MAX_S3_KEY_LEN {
        return Err(BackendError::InternalError(Box::new(
            S3Error::InvalidConfig(format!(
                "encoded S3 key length {} exceeds the {} byte limit",
                s3_key.len(),
                MAX_S3_KEY_LEN
            )),
        )));
    }

    Ok(s3_key)
}

#[async_trait::async_trait]
impl Backend for S3Backend {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        let k = encode_s3_key(&self.prefix, &self.key_format, key)?;
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&k)
            .send()
            .await;

        let resp = match result {
            Ok(r) => r,
            Err(e) => {
                let service_error = e.as_service_error();
                // A non-existent bucket is a misconfiguration, not a cache
                // miss. It must surface as an error (which the FSM logs on every
                // read) rather than masquerading as an empty cache forever.
                // `NoSuchBucket` is also an HTTP 404, so it has to be ruled out
                // before the generic-404 miss check below.
                let is_no_such_bucket =
                    service_error.and_then(|err| err.code()) == Some("NoSuchBucket");
                // A missing object IS a cache miss. Detect it via the typed
                // `NoSuchKey` variant *or* a raw HTTP 404, so non-AWS
                // S3-compatibles that return a generic 404 also work.
                let is_missing_key = !is_no_such_bucket
                    && (matches!(service_error, Some(GetObjectError::NoSuchKey(_)))
                        || e.raw_response().map(|r| r.status().as_u16()) == Some(404));
                if is_missing_key {
                    return Ok(None);
                }
                return Err(S3Error::from(e).into());
            }
        };

        let body = resp
            .body
            .collect()
            .await
            .map_err(|e| BackendError::from(S3Error::BodyRead(e.to_string())))?
            .into_bytes();

        // Decode + expiry are handled by the shared envelope policy: a corrupt
        // blob or an expired entry both read as a cache miss. `body` is owned
        // `Bytes`, so the payload is sliced out without copying.
        Ok(ValueEnvelope::decode_unexpired(body, self.label.as_str()))
    }

    async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
        let k = encode_s3_key(&self.prefix, &self.key_format, key)?;
        let body = ValueEnvelope::from(value).serialize()?;
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&k)
            .body(ByteStream::from(body))
            .send()
            .await
            .map_err(S3Error::from)?;
        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let k = encode_s3_key(&self.prefix, &self.key_format, key)?;
        // DeleteObject is idempotent and does not report whether the key
        // previously existed; always report `Deleted(1)`. Distinguishing
        // missing would require an extra HEAD request.
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&k)
            .send()
            .await
            .map_err(S3Error::from)?;
        Ok(DeleteStatus::Deleted(1))
    }

    fn label(&self) -> BackendLabel {
        self.label.clone()
    }

    fn value_format(&self) -> &dyn Format {
        self.value_format.as_ref()
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &self.key_format
    }

    fn compressor(&self) -> &dyn Compressor {
        self.compressor.as_ref()
    }
}

impl hitbox_backend::CacheBackend for S3Backend {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_key_empty_prefix_is_just_hex() {
        let key = CacheKey::from_str("user", "1");
        let s3_key = encode_s3_key("", &CacheKeyFormat::Bitcode, &key).unwrap();
        assert!(!s3_key.contains('/'));
        assert!(s3_key.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn encode_key_with_prefix() {
        let key = CacheKey::from_str("user", "1");
        let s3_key = encode_s3_key("cache", &CacheKeyFormat::Bitcode, &key).unwrap();
        assert!(s3_key.starts_with("cache/"));
        let hex_part = s3_key.strip_prefix("cache/").unwrap();
        assert!(hex_part.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn trailing_slash_prefix_does_not_double_slash() {
        let key = CacheKey::from_str("user", "1");
        let s3_key = encode_s3_key("cache/", &CacheKeyFormat::Bitcode, &key).unwrap();
        assert!(s3_key.starts_with("cache/"));
        assert!(!s3_key.contains("//"));
    }

    #[test]
    fn distinct_keys_produce_distinct_s3_keys() {
        let a = CacheKey::from_str("user", "1");
        let b = CacheKey::from_str("user", "2");
        let ka = encode_s3_key("p", &CacheKeyFormat::Bitcode, &a).unwrap();
        let kb = encode_s3_key("p", &CacheKeyFormat::Bitcode, &b).unwrap();
        assert_ne!(ka, kb);
    }

    #[test]
    fn different_key_formats_yield_different_hex() {
        // The S3 key is always hex-encoded, but the *input* bytes differ per
        // key format, so the hex output must differ too.
        let key = CacheKey::from_str("user", "1");
        let bitcode = encode_s3_key("", &CacheKeyFormat::Bitcode, &key).unwrap();
        let urlencoded = encode_s3_key("", &CacheKeyFormat::UrlEncoded, &key).unwrap();
        assert_ne!(bitcode, urlencoded);
    }

    #[test]
    fn long_key_stays_under_s3_limit() {
        let long = "x".repeat(400);
        let key = CacheKey::from_str(&long, "1");
        let s3_key = encode_s3_key("prefix", &CacheKeyFormat::Bitcode, &key).unwrap();
        assert!(s3_key.len() < MAX_S3_KEY_LEN);
    }

    #[test]
    fn oversize_key_is_rejected() {
        // Hex doubles the size; a ~700 byte prefix string serializes to > 512
        // bytes which hex-encodes to > 1024 bytes.
        let huge = "z".repeat(1200);
        let key = CacheKey::from_str(&huge, "1");
        let result = encode_s3_key("", &CacheKeyFormat::Bitcode, &key);
        assert!(result.is_err());
    }
}
