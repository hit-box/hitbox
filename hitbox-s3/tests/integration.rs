//! Integration tests for `hitbox-s3` against a MinIO container.
//!
//! These tests require Docker and are gated behind the `integration` feature:
//!
//! ```bash
//! cargo test -p hitbox-s3 --features integration
//! ```
//!
//! A single MinIO container is shared across all tests (started once via a
//! `OnceCell`); per-test isolation comes from unique key prefixes.
#![cfg(feature = "integration")]

use std::sync::atomic::{AtomicU64, Ordering};

use bytes::Bytes;
use chrono::Utc;
use hitbox_backend::{Backend, CacheBackend, DeleteStatus};
use hitbox_core::{CacheKey, CacheValue};
use hitbox_s3::S3Backend;
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::minio::MinIO;
use tokio::sync::OnceCell;

const BUCKET: &str = "hitbox-test-bucket";

/// Shared MinIO container plus its resolved endpoint URL. Kept alive for the
/// lifetime of the test binary so a single container serves every test.
struct MinioCtx {
    _node: ContainerAsync<MinIO>,
    endpoint: String,
}

static MINIO: OnceCell<MinioCtx> = OnceCell::const_new();
static PREFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn minio() -> &'static MinioCtx {
    MINIO
        .get_or_init(|| async {
            let node = MinIO::default()
                .start()
                .await
                .expect("failed to start MinIO container");
            let port = node
                .get_host_port_ipv4(9000)
                .await
                .expect("failed to resolve MinIO port");
            MinioCtx {
                _node: node,
                endpoint: format!("http://127.0.0.1:{port}"),
            }
        })
        .await
}

/// Builds a backend with a unique prefix and ensures the shared bucket exists.
async fn backend() -> S3Backend {
    let ctx = minio().await;
    let prefix = format!("t{}", PREFIX_COUNTER.fetch_add(1, Ordering::SeqCst));

    let backend = S3Backend::builder(BUCKET)
        .endpoint(&ctx.endpoint)
        .region("us-east-1")
        .credentials("minioadmin", "minioadmin")
        .prefix(prefix)
        .build()
        .await
        .expect("failed to build S3 backend");

    backend
        .ensure_bucket()
        .await
        .expect("failed to ensure bucket");
    backend
}

fn key(name: &str) -> CacheKey {
    CacheKey::from_str(name, "1")
}

#[tokio::test]
async fn read_write_remove_cycle() {
    let backend = backend().await;
    let k = key("rw-cycle");
    let value = CacheValue::new(
        Bytes::from_static(b"hello-s3"),
        Some(Utc::now() + chrono::Duration::hours(1)),
        None,
    );

    backend.write(&k, value).await.unwrap();

    let read = backend.read(&k).await.unwrap().expect("entry should exist");
    assert_eq!(read.data().as_ref(), b"hello-s3");

    let status = backend.remove(&k).await.unwrap();
    assert_eq!(status, DeleteStatus::Deleted(1));

    assert!(backend.read(&k).await.unwrap().is_none());
}

#[tokio::test]
async fn read_missing_key_is_none() {
    let backend = backend().await;
    let result = backend.read(&key("does-not-exist")).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn read_expired_entry_is_none() {
    let backend = backend().await;
    let k = key("expired");
    let value = CacheValue::new(
        Bytes::from_static(b"stale-bytes"),
        Some(Utc::now() - chrono::Duration::seconds(10)),
        None,
    );
    backend.write(&k, value).await.unwrap();

    // Lazy expiration: object exists but read returns None.
    assert!(backend.read(&k).await.unwrap().is_none());
}

#[tokio::test]
async fn read_unexpired_entry_returns_value() {
    let backend = backend().await;
    let k = key("fresh");
    let value = CacheValue::new(
        Bytes::from_static(b"fresh-bytes"),
        Some(Utc::now() + chrono::Duration::hours(1)),
        None,
    );
    backend.write(&k, value).await.unwrap();

    let read = backend.read(&k).await.unwrap().expect("should exist");
    assert_eq!(read.data().as_ref(), b"fresh-bytes");
}

#[tokio::test]
async fn write_overwrites_last_write_wins() {
    let backend = backend().await;
    let k = key("lww");

    backend
        .write(
            &k,
            CacheValue::new(Bytes::from_static(b"first"), None, None),
        )
        .await
        .unwrap();
    backend
        .write(
            &k,
            CacheValue::new(Bytes::from_static(b"second"), None, None),
        )
        .await
        .unwrap();

    let read = backend.read(&k).await.unwrap().expect("should exist");
    assert_eq!(read.data().as_ref(), b"second");
}

#[tokio::test]
async fn remove_missing_key_is_idempotent() {
    let backend = backend().await;
    let status = backend.remove(&key("never-written")).await.unwrap();
    assert_eq!(status, DeleteStatus::Deleted(1));
}

#[tokio::test]
async fn expire_and_stale_survive_roundtrip() {
    let backend = backend().await;
    let k = key("metadata");
    let expire = Utc::now() + chrono::Duration::hours(2);
    let stale = Utc::now() + chrono::Duration::hours(1);
    let value = CacheValue::new(Bytes::from_static(b"meta"), Some(expire), Some(stale));
    backend.write(&k, value).await.unwrap();

    let read = backend.read(&k).await.unwrap().expect("should exist");
    let tolerance = chrono::Duration::seconds(1);
    assert!((read.expire().unwrap() - expire).abs() < tolerance);
    assert!((read.stale().unwrap() - stale).abs() < tolerance);
}

#[tokio::test]
async fn stale_but_not_expired_is_still_readable() {
    // An entry past its `stale` time but before `expire` must still be
    // returned (so the FSM can run stale-while-revalidate).
    let backend = backend().await;
    let k = key("stale-readable");
    let value = CacheValue::new(
        Bytes::from_static(b"stale-while-revalidate"),
        Some(Utc::now() + chrono::Duration::hours(1)),
        Some(Utc::now() - chrono::Duration::seconds(10)),
    );
    backend.write(&k, value).await.unwrap();

    let read = backend.read(&k).await.unwrap().expect("should exist");
    assert_eq!(read.data().as_ref(), b"stale-while-revalidate");
    assert!(read.stale().is_some());
}

#[tokio::test]
async fn typed_get_set_roundtrip() {
    use hitbox_core::{BoxContext, CacheContext};

    let backend = backend().await;
    let k = key("typed");
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let value = CacheValue::new("typed-value".to_string(), None, None);

    backend.set::<String>(&k, &value, &mut ctx).await.unwrap();

    let read = backend
        .get::<String>(&k, &mut ctx)
        .await
        .unwrap()
        .expect("should exist");
    assert_eq!(read.into_inner(), "typed-value".to_string());
}

#[tokio::test]
async fn corrupt_object_reads_as_miss() {
    // Write a raw object whose body is not a valid envelope, then assert the
    // backend treats it as a cache miss rather than erroring.
    let ctx = minio().await;
    let prefix = format!("t{}", PREFIX_COUNTER.fetch_add(1, Ordering::SeqCst));
    let backend = S3Backend::builder(BUCKET)
        .endpoint(&ctx.endpoint)
        .region("us-east-1")
        .credentials("minioadmin", "minioadmin")
        .prefix(&prefix)
        .build()
        .await
        .unwrap();
    backend.ensure_bucket().await.unwrap();

    // Plant a garbage object directly, via a raw AWS client, at exactly the key
    // the backend computes for "corrupt", then assert the backend reads it as a
    // miss rather than erroring.
    let raw_key = {
        // Reproduce the backend's key derivation: prefix/hex(bitcode(key)).
        let serialized = hitbox_backend::CacheKeyFormat::Bitcode
            .serialize(&key("corrupt"))
            .unwrap();
        format!("{prefix}/{}", hex::encode(serialized))
    };

    let aws_conf = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new("us-east-1"))
        .endpoint_url(&ctx.endpoint)
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            "minioadmin",
            "minioadmin",
            None,
            None,
            "test",
        ))
        .load()
        .await;
    let s3 = aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::config::Builder::from(&aws_conf)
            .force_path_style(true)
            .build(),
    );
    s3.put_object()
        .bucket(BUCKET)
        .key(&raw_key)
        .body(aws_sdk_s3::primitives::ByteStream::from_static(
            b"not-an-envelope",
        ))
        .send()
        .await
        .unwrap();

    let result = backend.read(&key("corrupt")).await.unwrap();
    assert!(result.is_none(), "corrupt object should read as a miss");
}

#[tokio::test]
async fn read_on_missing_bucket_is_error() {
    // A non-existent bucket is a misconfiguration, not a cache miss: read()
    // must return an error (which the FSM logs) instead of a silent Ok(None)
    // that would look like a permanently empty cache.
    let ctx = minio().await;
    let backend = S3Backend::builder("hitbox-nonexistent-bucket")
        .endpoint(&ctx.endpoint)
        .region("us-east-1")
        .credentials("minioadmin", "minioadmin")
        .build()
        .await
        .unwrap();
    // Deliberately do NOT create the bucket.

    let result = backend.read(&key("anything")).await;
    assert!(
        result.is_err(),
        "read on a non-existent bucket must be an error, not Ok(None)"
    );
}

#[tokio::test]
async fn subsecond_expire_survives_roundtrip_exactly() {
    // The 1s-tolerance test above wouldn't catch whole-second truncation; the
    // envelope stores full nanosecond precision, so assert an exact round-trip.
    let backend = backend().await;
    let k = key("subsecond");
    let expire = Utc::now() + chrono::Duration::milliseconds(1500);
    let value = CacheValue::new(Bytes::from_static(b"precise"), Some(expire), None);
    backend.write(&k, value).await.unwrap();

    let read = backend.read(&k).await.unwrap().expect("should exist");
    assert_eq!(read.expire(), Some(expire));
}
