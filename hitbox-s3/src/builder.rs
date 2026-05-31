//! Builder for [`S3Backend`].

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Credentials, Region};
use hitbox_backend::format::{BincodeFormat, Format};
use hitbox_backend::{CacheKeyFormat, Compressor, PassthroughCompressor};
use hitbox_core::BackendLabel;

use crate::backend::S3Backend;
use crate::error::S3Error;

/// Builder for [`S3Backend`].
///
/// The only required field is the bucket name (passed to
/// [`S3Backend::builder`]). All other settings are optional:
///
/// - `prefix` — key prefix for all objects (default: none)
/// - `endpoint` — custom endpoint URL for S3-compatible services (MinIO, R2,
///   B2). Setting it also enables path-style addressing.
/// - `region` — AWS region (default: resolved from the environment, or
///   `us-east-1` when a custom `endpoint` is set)
/// - `credentials` — static access key / secret (default: resolved from the
///   environment / default credential chain)
/// - `label` — backend label for metrics and composition (default: `"s3"`)
/// - `value_format` — value serialization (default: [`BincodeFormat`])
/// - `key_format` — cache key serialization (default:
///   [`CacheKeyFormat::Bitcode`])
/// - `compressor` — value compression (default: [`PassthroughCompressor`])
///
/// # Example
///
/// ```no_run
/// use hitbox_s3::S3Backend;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = S3Backend::builder("my-cache-bucket")
///     .prefix("hitbox")
///     .region("us-east-1")
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct S3BackendBuilder {
    bucket: String,
    prefix: Option<String>,
    endpoint: Option<String>,
    region: Option<String>,
    credentials: Option<(String, String)>,
    label: BackendLabel,
    value_format: Box<dyn Format + Send + Sync>,
    key_format: CacheKeyFormat,
    compressor: Box<dyn Compressor + Send + Sync>,
}

impl S3BackendBuilder {
    /// Creates a new builder for the given bucket.
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            prefix: None,
            endpoint: None,
            region: None,
            credentials: None,
            label: BackendLabel::new_static("s3"),
            value_format: Box::new(BincodeFormat),
            key_format: CacheKeyFormat::Bitcode,
            compressor: Box::new(PassthroughCompressor),
        }
    }

    /// Sets a key prefix prepended to every object key.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets a custom endpoint URL (for MinIO, Cloudflare R2, Backblaze B2,
    /// etc.). This also enables path-style addressing.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Sets the AWS region.
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Sets static credentials (access key id + secret access key).
    ///
    /// Useful for MinIO / R2 / B2. When omitted, the default AWS credential
    /// chain is used (environment, profile, IAM role, etc.).
    pub fn credentials(
        mut self,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        self.credentials = Some((access_key_id.into(), secret_access_key.into()));
        self
    }

    /// Sets the backend label for metrics and multi-tier composition.
    pub fn label(mut self, label: impl Into<BackendLabel>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the value serialization format. Default: [`BincodeFormat`].
    pub fn value_format<F>(mut self, format: F) -> Self
    where
        F: Format + Send + Sync + 'static,
    {
        // `format` is owned here; box it directly rather than `clone_box`-ing.
        self.value_format = Box::new(format);
        self
    }

    /// Sets the cache key serialization format. Default:
    /// [`CacheKeyFormat::Bitcode`].
    pub fn key_format(mut self, format: CacheKeyFormat) -> Self {
        self.key_format = format;
        self
    }

    /// Sets the value compressor. Default: [`PassthroughCompressor`].
    pub fn compressor<C>(mut self, compressor: C) -> Self
    where
        C: Compressor + Send + Sync + 'static,
    {
        self.compressor = Box::new(compressor);
        self
    }

    /// Builds the [`S3Backend`], loading AWS configuration and constructing the
    /// S3 client.
    ///
    /// # Errors
    ///
    /// Returns [`S3Error::InvalidConfig`] if the bucket name is empty.
    pub async fn build(self) -> Result<S3Backend, S3Error> {
        if self.bucket.is_empty() {
            return Err(S3Error::InvalidConfig(
                "bucket name must not be empty".into(),
            ));
        }

        // For a custom endpoint (MinIO/R2/B2) the region is functionally
        // irrelevant, but the SDK still requires one — fall back to us-east-1 so
        // an endpoint-only setup doesn't fail region resolution.
        let has_endpoint = self.endpoint.is_some();
        let region = self
            .region
            .or_else(|| has_endpoint.then(|| "us-east-1".to_string()));

        let mut loader = aws_config::defaults(BehaviorVersion::latest());

        if let Some(region) = region {
            loader = loader.region(Region::new(region));
        }
        if let Some((access, secret)) = self.credentials {
            loader = loader.credentials_provider(Credentials::new(
                access,
                secret,
                None,
                None,
                "hitbox-s3-static",
            ));
        }
        if let Some(endpoint) = self.endpoint {
            loader = loader.endpoint_url(endpoint);
        }

        let shared_config = loader.load().await;

        let mut s3_config = aws_sdk_s3::config::Builder::from(&shared_config);
        // Path-style addressing is required for most S3-compatible services
        // (MinIO/R2/B2) reached through a custom endpoint.
        if has_endpoint {
            s3_config = s3_config.force_path_style(true);
        }

        let client = Client::from_conf(s3_config.build());

        Ok(S3Backend {
            client,
            bucket: self.bucket,
            prefix: self.prefix.unwrap_or_default(),
            label: self.label,
            value_format: self.value_format,
            key_format: self.key_format,
            compressor: self.compressor,
        })
    }
}
