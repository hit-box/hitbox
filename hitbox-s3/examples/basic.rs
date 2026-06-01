//! Minimal `hitbox-s3` usage against an S3-compatible endpoint (e.g. MinIO).
//!
//! Run a local MinIO first, for example:
//!
//! ```bash
//! docker run -p 9000:9000 -p 9001:9001 \
//!   -e MINIO_ROOT_USER=minioadmin -e MINIO_ROOT_PASSWORD=minioadmin \
//!   minio/minio server /data --console-address ":9001"
//! ```
//!
//! Then:
//!
//! ```bash
//! cargo run -p hitbox-s3 --example basic --features test-utils
//! ```

use bytes::Bytes;
use chrono::Utc;
use hitbox_backend::Backend;
use hitbox_core::{CacheKey, CacheValue};
use hitbox_s3::S3Backend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = S3Backend::builder("hitbox-example")
        .endpoint("http://127.0.0.1:9000")
        .region("us-east-1")
        .credentials("minioadmin", "minioadmin")
        .prefix("demo")
        .build()
        .await?;

    // `ensure_bucket` is gated behind the `test-utils` feature; in production
    // you would provision the bucket out of band.
    #[cfg(feature = "test-utils")]
    backend.ensure_bucket().await?;

    let key = CacheKey::from_str("greeting", "1");
    let value = CacheValue::new(
        Bytes::from_static(b"hello from hitbox-s3"),
        Some(Utc::now() + chrono::Duration::minutes(5)),
        None,
    );

    backend.write(&key, value).await?;
    println!("wrote entry");

    match backend.read(&key).await? {
        Some(found) => println!("read back: {:?}", String::from_utf8_lossy(found.data())),
        None => println!("cache miss"),
    }

    backend.remove(&key).await?;
    println!("removed entry");

    Ok(())
}
