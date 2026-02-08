//! Redis backend implementation.

use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use hitbox::{BackendLabel, CacheKey, CacheValue, Raw};
use hitbox_backend::{
    Backend, BackendError, BackendResult, CacheKeyFormat, Compressor, DeleteStatus,
    PassthroughCompressor,
    format::{BincodeFormat, Format},
};
use redis::Client;
use redis::aio::ConnectionManager;
#[cfg(feature = "cluster")]
use redis::cluster_async::ClusterConnection;
use tokio::sync::OnceCell;

use crate::error::Error;

/// Configuration for a single Redis node connection.
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It appears when:
/// - Using [`ConnectionMode::single`] which creates this internally
/// - Accessing configuration for debugging or logging
///
/// # Examples
///
/// ```
/// use hitbox_redis::SingleConfig;
///
/// let config = SingleConfig::new("redis://localhost:6379/");
/// ```
#[derive(Debug, Clone)]
pub struct SingleConfig {
    /// Redis connection URL in format `redis://[:<password>@]<host>[:<port>][/<database>]`.
    pub(crate) url: String,
    /// Exponential backoff base for reconnection attempts. Default: `2.0`.
    pub(crate) exponent_base: f32,
}

impl SingleConfig {
    /// Creates a new single-node configuration.
    ///
    /// # Arguments
    ///
    /// * `url` - Redis connection URL in format `redis://[:<password>@]<host>[:<port>][/<database>]`
    ///
    /// # Default
    ///
    /// * `exponent_base`: `2.0` (exponential backoff base for retries)
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_redis::SingleConfig;
    ///
    /// let config = SingleConfig::new("redis://localhost:6379/0");
    /// ```
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            exponent_base: 2.0,
        }
    }
}

/// Configuration for a Redis Cluster connection.
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It appears when:
/// - Using [`ConnectionMode::cluster`] which creates this internally
/// - Accessing configuration for debugging or logging
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "cluster")]
/// # fn main() {
/// use hitbox_redis::ClusterConfig;
///
/// let config = ClusterConfig::new([
///     "redis://node1:6379",
///     "redis://node2:6379",
///     "redis://node3:6379",
/// ]);
/// # }
/// # #[cfg(not(feature = "cluster"))]
/// # fn main() {}
/// ```
#[cfg(feature = "cluster")]
#[cfg_attr(docsrs, doc(cfg(feature = "cluster")))]
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// List of initial cluster node URLs. The client discovers other nodes automatically.
    pub(crate) nodes: Vec<String>,
    /// Whether to allow reading from replica nodes. Default: `false`.
    pub(crate) read_from_replicas: bool,
}

#[cfg(feature = "cluster")]
impl ClusterConfig {
    /// Creates a new cluster configuration.
    ///
    /// # Arguments
    ///
    /// * `nodes` - List of initial cluster node URLs. The client discovers
    ///   other nodes automatically via the `CLUSTER SLOTS` command.
    ///
    /// # Default
    ///
    /// * `read_from_replicas`: `false`
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "cluster")]
    /// # fn main() {
    /// use hitbox_redis::ClusterConfig;
    ///
    /// let config = ClusterConfig::new([
    ///     "redis://node1:6379",
    ///     "redis://node2:6379",
    ///     "redis://node3:6379",
    /// ]);
    /// # }
    /// # #[cfg(not(feature = "cluster"))]
    /// # fn main() {}
    /// ```
    pub fn new<I, S>(nodes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            nodes: nodes.into_iter().map(Into::into).collect(),
            read_from_replicas: false,
        }
    }
}

/// Redis connection mode.
///
/// Determines whether to connect to a single Redis instance or a Redis Cluster.
///
/// # Examples
///
/// Single-node connection:
/// ```
/// use hitbox_redis::ConnectionMode;
///
/// let mode = ConnectionMode::single("redis://localhost:6379/");
/// ```
///
/// Cluster connection (requires `cluster` feature):
///
/// ```
/// # #[cfg(feature = "cluster")]
/// # fn main() {
/// use hitbox_redis::ConnectionMode;
///
/// let mode = ConnectionMode::cluster([
///     "redis://node1:6379",
///     "redis://node2:6379",
///     "redis://node3:6379",
/// ]);
/// # }
/// # #[cfg(not(feature = "cluster"))]
/// # fn main() {}
/// ```
#[derive(Debug, Clone)]
pub enum ConnectionMode {
    /// Single Redis node connection.
    Single(SingleConfig),

    /// Redis Cluster connection.
    #[cfg(feature = "cluster")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cluster")))]
    Cluster(ClusterConfig),
}

impl ConnectionMode {
    /// Create a single-node connection mode.
    ///
    /// # Arguments
    ///
    /// * `url` - Redis connection URL in format `redis://[:<password>@]<host>[:<port>][/<database>]`
    pub fn single(url: impl Into<String>) -> Self {
        Self::Single(SingleConfig::new(url))
    }

    /// Create a cluster connection mode.
    ///
    /// # Arguments
    ///
    /// * `nodes` - List of initial cluster node URLs. The client will discover other nodes automatically.
    #[cfg(feature = "cluster")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cluster")))]
    pub fn cluster<I, S>(nodes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Cluster(ClusterConfig::new(nodes))
    }

    /// Sets the exponential backoff base for retries (single-node only).
    ///
    /// The delay between reconnection attempts is calculated as `base^attempt` milliseconds.
    ///
    /// # Default
    ///
    /// `2.0`
    ///
    /// # Caveats
    ///
    /// This option only applies to single-node connections and is silently ignored
    /// for cluster mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_redis::ConnectionMode;
    ///
    /// // Use a slower backoff (3^attempt ms)
    /// let mode = ConnectionMode::single("redis://localhost:6379/")
    ///     .exponent_base(3.0);
    /// ```
    #[allow(irrefutable_let_patterns)]
    pub fn exponent_base(mut self, base: f32) -> Self {
        if let Self::Single(ref mut config) = self {
            config.exponent_base = base;
        }
        self
    }

    /// Enables reading from replica nodes (cluster only).
    ///
    /// When enabled, read operations may be served by replica nodes for better
    /// read throughput and reduced load on primary nodes.
    ///
    /// # Default
    ///
    /// Disabled (reads only from primary nodes).
    ///
    /// # Caveats
    ///
    /// - Replicas may have slightly stale data due to replication lag
    /// - This option only applies to cluster connections and is silently ignored
    ///   for single-node mode
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "cluster")]
    /// # fn main() {
    /// use hitbox_redis::ConnectionMode;
    ///
    /// let mode = ConnectionMode::cluster([
    ///     "redis://node1:6379",
    ///     "redis://node2:6379",
    /// ])
    /// .read_from_replicas();
    /// # }
    /// # #[cfg(not(feature = "cluster"))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "cluster")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cluster")))]
    pub fn read_from_replicas(mut self) -> Self {
        if let Self::Cluster(ref mut config) = self {
            config.read_from_replicas = true;
        }
        self
    }
}

/// Internal wrapper for Redis connection types.
#[derive(Clone)]
enum RedisConnection {
    Single(ConnectionManager),
    #[cfg(feature = "cluster")]
    Cluster(ClusterConnection),
}

impl RedisConnection {
    /// Execute a pipeline and return the result.
    async fn query_pipeline<T: redis::FromRedisValue>(
        &mut self,
        pipe: &redis::Pipeline,
    ) -> Result<T, redis::RedisError> {
        match self {
            Self::Single(conn) => pipe.query_async(conn).await,
            #[cfg(feature = "cluster")]
            Self::Cluster(conn) => pipe.query_async(conn).await,
        }
    }

    /// Execute a single command.
    async fn query_cmd<T: redis::FromRedisValue>(
        &mut self,
        cmd: &mut redis::Cmd,
    ) -> Result<T, redis::RedisError> {
        match self {
            Self::Single(conn) => cmd.query_async(conn).await,
            #[cfg(feature = "cluster")]
            Self::Cluster(conn) => cmd.query_async(conn).await,
        }
    }
}

/// Redis cache backend for single-node or cluster deployments.
///
/// `RedisBackend` provides a cache backend using Redis as the storage layer.
/// It supports both single-node Redis instances and Redis Cluster
/// (with the `cluster` feature enabled).
///
/// Use [`RedisBackendBuilder`] to construct this type.
///
/// # Type Parameters
///
/// * `S` - Serialization format for cache values. Implements [`Format`].
///   Default: [`BincodeFormat`] (compact binary, recommended for production).
/// * `C` - Compression strategy for cache values. Implements [`Compressor`].
///   Default: [`PassthroughCompressor`] (no compression).
///
/// # Examples
///
/// Basic single-node connection:
///
/// ```
/// use hitbox_redis::{RedisBackend, ConnectionMode};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = RedisBackend::builder()
///     .connection(ConnectionMode::single("redis://localhost:6379/"))
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// With all configuration options:
///
/// ```
/// use std::time::Duration;
/// use hitbox_redis::{RedisBackend, ConnectionMode};
/// use hitbox_backend::CacheKeyFormat;
/// use hitbox_backend::format::JsonFormat;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = RedisBackend::builder()
///     .connection(ConnectionMode::single("redis://localhost:6379/"))
///     .username("cache_user")        // Redis 6+ ACL
///     .password("secret")
///     .label("user-sessions")
///     .key_format(CacheKeyFormat::UrlEncoded)
///     .value_format(JsonFormat)
///     .connection_timeout(Duration::from_secs(5))
///     .response_timeout(Duration::from_secs(2))
///     .retries(3)
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// Cluster connection (requires `cluster` feature):
///
/// ```
/// # #[cfg(feature = "cluster")]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use hitbox_redis::{RedisBackend, ConnectionMode};
///
/// let backend = RedisBackend::builder()
///     .connection(ConnectionMode::cluster([
///         "redis://node1:6379",
///         "redis://node2:6379",
///         "redis://node3:6379",
///     ]))
///     .build()?;
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "cluster"))]
/// # fn main() {}
/// ```
///
/// # Performance
///
/// - **Read operations**: Single pipelined request (`HMGET` + `PTTL`)
/// - **Write operations**: Single pipelined request (`HSET` + `PEXPIRE`)
/// - **Connection**: Established lazily on first use, multiplexed for concurrent access
///
/// # Caveats
///
/// - **Connection failure**: First cache operation will fail if Redis is unreachable
/// - **Expire time approximation**: The `expire` timestamp returned on read is
///   calculated as `now + PTTL`, which may drift by the network round-trip time
///
/// [`Format`]: hitbox_backend::format::Format
/// [`BincodeFormat`]: hitbox_backend::format::BincodeFormat
/// [`Compressor`]: hitbox_backend::Compressor
/// [`PassthroughCompressor`]: hitbox_backend::PassthroughCompressor
#[derive(Clone)]
pub struct RedisBackend<S = BincodeFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    /// Connection mode (single node or cluster).
    mode: ConnectionMode,
    /// Timeout for establishing connections.
    connection_timeout: Option<Duration>,
    /// Timeout for waiting on Redis responses.
    response_timeout: Option<Duration>,
    /// Maximum number of retry attempts.
    number_of_retries: Option<usize>,
    /// Username for Redis authentication (Redis 6+ ACL).
    username: Option<String>,
    /// Password for Redis authentication.
    password: Option<String>,

    /// Lazy-initialized connection (established on first cache operation).
    connection: OnceCell<RedisConnection>,

    /// Format used to serialize cache values.
    serializer: S,
    /// Format used to serialize cache keys.
    key_format: CacheKeyFormat,
    /// Compressor used for cache values.
    compressor: C,
    /// Label identifying this backend in multi-tier compositions.
    label: BackendLabel,
}

impl RedisBackend<BincodeFormat, PassthroughCompressor> {
    /// Creates a new builder for `RedisBackend`.
    ///
    /// Use the builder to configure the connection mode, serialization format,
    /// key format, compression, and label. See [`RedisBackend`] for examples.
    #[must_use]
    pub fn builder() -> RedisBackendBuilder<BincodeFormat, PassthroughCompressor> {
        RedisBackendBuilder::default()
    }
}

impl<S, C> RedisBackend<S, C>
where
    S: Format,
    C: Compressor,
{
    /// Gets or initializes the Redis connection lazily.
    ///
    /// This method ensures the connection is established only once, even when
    /// called concurrently from multiple tasks. Subsequent calls return the
    /// cached connection.
    async fn get_connection(&self) -> Result<&RedisConnection, Error> {
        self.connection
            .get_or_try_init(|| async {
                match &self.mode {
                    ConnectionMode::Single(config) => {
                        // Parse URL and apply authentication if provided
                        let mut conn_info: redis::ConnectionInfo = config.url.as_str().parse()?;
                        let mut redis_info = conn_info.redis_settings().clone();
                        if let Some(ref username) = self.username {
                            redis_info = redis_info.set_username(username);
                        }
                        if let Some(ref password) = self.password {
                            redis_info = redis_info.set_password(password);
                        }
                        conn_info = conn_info.set_redis_settings(redis_info);

                        let client = Client::open(conn_info)?;

                        // Build ConnectionManagerConfig with options
                        let mut manager_config = redis::aio::ConnectionManagerConfig::new()
                            .set_exponent_base(config.exponent_base);

                        if let Some(timeout) = self.connection_timeout {
                            manager_config = manager_config.set_connection_timeout(Some(timeout));
                        }
                        if let Some(timeout) = self.response_timeout {
                            manager_config = manager_config.set_response_timeout(Some(timeout));
                        }
                        if let Some(retries) = self.number_of_retries {
                            manager_config = manager_config.set_number_of_retries(retries);
                        }

                        let conn = client
                            .get_connection_manager_with_config(manager_config)
                            .await?;
                        Ok(RedisConnection::Single(conn))
                    }
                    #[cfg(feature = "cluster")]
                    ConnectionMode::Cluster(config) => {
                        let mut builder = redis::cluster::ClusterClientBuilder::new(
                            config.nodes.iter().map(|s| s.as_str()),
                        );
                        if config.read_from_replicas {
                            builder = builder.read_from_replicas();
                        }
                        if let Some(ref username) = self.username {
                            builder = builder.username(username.clone());
                        }
                        if let Some(ref password) = self.password {
                            builder = builder.password(password.clone());
                        }
                        if let Some(timeout) = self.connection_timeout {
                            builder = builder.connection_timeout(timeout);
                        }
                        if let Some(timeout) = self.response_timeout {
                            builder = builder.response_timeout(timeout);
                        }
                        if let Some(retries) = self.number_of_retries {
                            builder = builder.retries(retries as u32);
                        }

                        let client = builder.build()?;
                        let conn = client.get_async_connection().await?;
                        Ok(RedisConnection::Cluster(conn))
                    }
                }
            })
            .await
    }
}

/// Builder for creating and configuring a [`RedisBackend`].
///
/// Use [`RedisBackend::builder`] to create a new builder instance.
/// See [`RedisBackend`] for usage examples.
pub struct RedisBackendBuilder<S = BincodeFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    mode: Option<ConnectionMode>,
    serializer: S,
    key_format: CacheKeyFormat,
    compressor: C,
    label: BackendLabel,
    // Common connection options
    connection_timeout: Option<Duration>,
    response_timeout: Option<Duration>,
    number_of_retries: Option<usize>,
    // Authentication
    username: Option<String>,
    password: Option<String>,
}

impl Default for RedisBackendBuilder<BincodeFormat, PassthroughCompressor> {
    fn default() -> Self {
        Self {
            mode: None,
            serializer: BincodeFormat,
            key_format: CacheKeyFormat::default(),
            compressor: PassthroughCompressor,
            label: BackendLabel::new_static("redis"),
            connection_timeout: None,
            response_timeout: None,
            number_of_retries: None,
            username: None,
            password: None,
        }
    }
}

impl<S, C> RedisBackendBuilder<S, C>
where
    S: Format,
    C: Compressor,
{
    /// Sets the Redis connection mode.
    ///
    /// This is required before calling [`build`].
    ///
    /// [`build`]: Self::build
    pub fn connection(mut self, mode: ConnectionMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Sets the connection timeout.
    ///
    /// This timeout applies when establishing a new connection to Redis.
    /// If the connection cannot be established within this duration, the operation fails.
    ///
    /// # Default
    ///
    /// No timeout (waits indefinitely).
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    /// Sets the response timeout.
    ///
    /// This timeout applies when waiting for a response from Redis after sending a command.
    /// If Redis doesn't respond within this duration, the operation fails.
    ///
    /// # Default
    ///
    /// No timeout (waits indefinitely).
    ///
    /// # Note
    ///
    /// If you use blocking commands (like `BLPOP`) or long-running commands,
    /// ensure the timeout is long enough to accommodate them.
    pub fn response_timeout(mut self, timeout: Duration) -> Self {
        self.response_timeout = Some(timeout);
        self
    }

    /// Sets the maximum number of connection retry attempts.
    ///
    /// When a connection fails, the client will retry up to this many times
    /// with exponential backoff before giving up.
    ///
    /// # Default
    ///
    /// Uses the redis-rs default (typically 16 retries for single-node).
    pub fn retries(mut self, count: usize) -> Self {
        self.number_of_retries = Some(count);
        self
    }

    /// Sets the username for Redis authentication.
    ///
    /// Used with Redis 6+ ACL system. For older Redis versions using only
    /// password authentication, leave this unset.
    ///
    /// # Default
    ///
    /// None (no username).
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Sets the password for Redis authentication.
    ///
    /// Works with both legacy Redis AUTH and Redis 6+ ACL authentication.
    /// For ACL authentication, also set [`username`](Self::username).
    ///
    /// # Default
    ///
    /// None (no password).
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Sets the cache value serialization format.
    ///
    /// The value format determines how cached data is serialized before storage.
    ///
    /// # Default
    ///
    /// [`BincodeFormat`] (compact binary, recommended for production)
    ///
    /// # Options
    ///
    /// | Format | Speed | Size | Human-readable |
    /// |--------|-------|------|----------------|
    /// | [`BincodeFormat`] | Fast | Compact | No |
    /// | [`JsonFormat`](hitbox_backend::format::JsonFormat) | Slow | Large | Yes |
    /// | [`RonFormat`](hitbox_backend::format::RonFormat) | Medium | Medium | Yes |
    ///
    /// [`BincodeFormat`]: hitbox_backend::format::BincodeFormat
    pub fn value_format<NewS>(self, serializer: NewS) -> RedisBackendBuilder<NewS, C>
    where
        NewS: Format,
    {
        RedisBackendBuilder {
            mode: self.mode,
            serializer,
            key_format: self.key_format,
            compressor: self.compressor,
            label: self.label,
            connection_timeout: self.connection_timeout,
            response_timeout: self.response_timeout,
            number_of_retries: self.number_of_retries,
            username: self.username,
            password: self.password,
        }
    }

    /// Sets the cache key serialization format.
    ///
    /// The key format determines how [`CacheKey`] values are serialized for
    /// storage as Redis keys. This affects key size and debuggability.
    ///
    /// # Default
    ///
    /// [`CacheKeyFormat::Bitcode`]
    ///
    /// # Options
    ///
    /// | Format | Size | Human-readable |
    /// |--------|------|----------------|
    /// | [`Bitcode`](CacheKeyFormat::Bitcode) | Compact | No |
    /// | [`UrlEncoded`](CacheKeyFormat::UrlEncoded) | Larger | Yes |
    ///
    /// [`CacheKey`]: hitbox::CacheKey
    pub fn key_format(mut self, key_format: CacheKeyFormat) -> Self {
        self.key_format = key_format;
        self
    }

    /// Sets a custom label for this backend.
    ///
    /// The label identifies this backend in multi-tier cache compositions and
    /// appears in metrics and debug output.
    ///
    /// # Default
    ///
    /// `"redis"`
    pub fn label(mut self, label: impl Into<BackendLabel>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the compression strategy for cache values.
    ///
    /// Compression reduces network bandwidth and Redis memory usage at the cost
    /// of CPU time. For Redis backends, compression is often beneficial since
    /// network I/O is typically the bottleneck.
    ///
    /// # Default
    ///
    /// [`PassthroughCompressor`] (no compression)
    ///
    /// # Options
    ///
    /// | Compressor | Ratio | Speed | Feature flag |
    /// |------------|-------|-------|--------------|
    /// | [`PassthroughCompressor`] | None | Fastest | — |
    /// | [`GzipCompressor`] | Good | Medium | `gzip` |
    /// | [`ZstdCompressor`] | Best | Fast | `zstd` |
    ///
    /// # When to Use Compression
    ///
    /// - Cached values larger than ~1KB
    /// - High network latency to Redis
    /// - Redis memory is constrained
    /// - Using Redis persistence (RDB/AOF)
    ///
    /// [`PassthroughCompressor`]: hitbox_backend::PassthroughCompressor
    /// [`GzipCompressor`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.GzipCompressor.html
    /// [`ZstdCompressor`]: https://docs.rs/hitbox-backend/latest/hitbox_backend/struct.ZstdCompressor.html
    pub fn compressor<NewC>(self, compressor: NewC) -> RedisBackendBuilder<S, NewC>
    where
        NewC: Compressor,
    {
        RedisBackendBuilder {
            mode: self.mode,
            serializer: self.serializer,
            key_format: self.key_format,
            compressor,
            label: self.label,
            connection_timeout: self.connection_timeout,
            response_timeout: self.response_timeout,
            number_of_retries: self.number_of_retries,
            username: self.username,
            password: self.password,
        }
    }

    /// Builds the [`RedisBackend`] with the configured settings.
    ///
    /// This method is synchronous - the actual Redis connection is established
    /// lazily on first use (get/set/delete operation).
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingConnectionMode`] if no connection mode was specified.
    /// Note: Connection errors will occur on first cache operation, not here.
    ///
    /// [`Error::MissingConnectionMode`]: crate::error::Error::MissingConnectionMode
    pub fn build(self) -> Result<RedisBackend<S, C>, Error> {
        let mode = self.mode.ok_or(Error::MissingConnectionMode)?;

        Ok(RedisBackend {
            mode,
            connection_timeout: self.connection_timeout,
            response_timeout: self.response_timeout,
            number_of_retries: self.number_of_retries,
            username: self.username,
            password: self.password,
            connection: OnceCell::new(),
            serializer: self.serializer,
            key_format: self.key_format,
            compressor: self.compressor,
            label: self.label,
        })
    }
}

#[async_trait]
impl<S, C> Backend for RedisBackend<S, C>
where
    S: Format + Send + Sync,
    C: Compressor + Send + Sync,
{
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        let mut con = self.get_connection().await?.clone();
        let cache_key = self.key_format.serialize(key)?;

        // Pipeline: HMGET (data, stale) + PTTL with typed decoding
        let ((data, stale_ms), pttl): ((Option<Vec<u8>>, Option<i64>), i64) = con
            .query_pipeline(
                redis::pipe()
                    .cmd("HMGET")
                    .arg(&cache_key)
                    .arg("d")
                    .arg("s")
                    .cmd("PTTL")
                    .arg(&cache_key),
            )
            .await
            .map_err(Error::from)?;

        // If data is None, key doesn't exist
        let data = match data {
            Some(data) => Bytes::from(data),
            None => return Ok(None),
        };

        // Convert stale millis to DateTime
        let stale = stale_ms.and_then(DateTime::from_timestamp_millis);

        // Calculate expire from PTTL (milliseconds remaining)
        // PTTL returns: -2 if key doesn't exist, -1 if no TTL, else milliseconds
        let expire = (pttl > 0).then(|| Utc::now() + chrono::Duration::milliseconds(pttl));

        Ok(Some(CacheValue::new(data, expire, stale)))
    }

    async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
        let mut con = self.get_connection().await?.clone();
        let cache_key = self.key_format.serialize(key)?;

        // Build HSET command with data field, optionally add stale field
        let mut cmd = redis::cmd("HSET");
        cmd.arg(&cache_key).arg("d").arg(value.data().as_ref());
        if let Some(stale) = value.stale() {
            cmd.arg("s").arg(stale.timestamp_millis());
        }

        // Pipeline: HSET + optional PEXPIRE (computed from value.ttl())
        let mut pipe = redis::pipe();
        pipe.add_command(cmd).ignore();
        if let Some(ttl_duration) = value.ttl() {
            pipe.cmd("PEXPIRE")
                .arg(&cache_key)
                .arg(u64::try_from(ttl_duration.as_millis()).map_err(|_| {
                    BackendError::InternalError(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "TTL overflow: {}ms exceeds u64 range",
                            ttl_duration.as_millis()
                        ),
                    )))
                })?)
                .ignore();
        }

        con.query_pipeline::<()>(&pipe).await.map_err(Error::from)?;
        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let mut con = self.get_connection().await?.clone();
        let cache_key = self.key_format.serialize(key)?;

        let deleted: i32 = con
            .query_cmd(redis::cmd("DEL").arg(cache_key))
            .await
            .map_err(Error::from)?;

        if deleted > 0 {
            Ok(DeleteStatus::Deleted(deleted as u32))
        } else {
            Ok(DeleteStatus::Missing)
        }
    }

    fn label(&self) -> BackendLabel {
        self.label.clone()
    }

    fn value_format(&self) -> &dyn Format {
        &self.serializer
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &self.key_format
    }

    fn compressor(&self) -> &dyn Compressor {
        &self.compressor
    }
}

// Explicit CacheBackend implementation using default trait methods
impl<S, C> hitbox_backend::CacheBackend for RedisBackend<S, C>
where
    S: Format + Send + Sync,
    C: Compressor + Send + Sync,
{
}
