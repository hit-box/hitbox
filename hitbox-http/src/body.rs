//! HTTP body buffering and streaming utilities for transparent caching.
//!
//! # Design Rationale: Cache Layer Transparency
//!
//! Hitbox aims to be a transparent caching layer - upstream services and clients
//! should see the same behavior with or without the cache.
//!
//! ## Why `BufferedBody::Partial` exists
//!
//! When predicates inspect request/response bodies, they consume bytes from the stream.
//! To maintain transparency, these bytes must be forwarded to upstream. Since you can't
//! "un-read" from a stream, we must:
//!
//! 1. Buffer the consumed prefix
//! 2. Preserve the unconsumed remaining stream
//! 3. Replay prefix + remaining to upstream
//!
//! This enables:
//! - Resumable uploads/downloads (e.g., large file transfers)
//! - Accurate error reporting (errors occur at the same byte position)
//! - Zero data loss or corruption
//! - Support for partial transfer protocols (HTTP Range requests, etc.)
//!
//! ## Example: Large File Upload
//!
//! ```text
//! Without cache:
//!   Client → Upstream: 500MB uploaded, error at byte 300MB
//!
//! With transparent cache:
//!   Client → Hitbox (reads 10MB for predicate) → Upstream
//!   Upstream receives: 10MB (replayed) + 290MB (streamed) + error
//!   Total: Same 300MB, same error position ✅
//! ```
//!
//! ## Body States
//!
//! - **Complete**: Body was fully read and buffered (within configured size limits)
//! - **Partial**: Body was partially read - contains buffered prefix plus remaining stream or error
//! - **Passthrough**: Body was not inspected at all (zero overhead)
//!
//! The `Partial` state is critical for maintaining transparency when:
//! - Body size exceeds configured limits but must still be forwarded
//! - Network or upstream errors occur mid-stream
//! - Predicates need to inspect body content without blocking large transfers

use bytes::{Buf, Bytes, BytesMut};
use http_body::{Body as HttpBody, Frame};
use pin_project::pin_project;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

/// What remains of a body stream after partial consumption.
///
/// When predicates or extractors read bytes from a body, the stream may have
/// more data available or may have encountered an error. This enum captures
/// both possibilities, preserving the stream state for forwarding to upstream.
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It appears when:
/// - Using [`BufferedBody::collect_exact`] which returns remaining stream data
/// - Decomposing a [`PartialBufferedBody`] via [`into_parts`](PartialBufferedBody::into_parts)
///
/// # Invariants
///
/// - `Body(stream)`: The stream has not ended and may yield more frames
/// - `Error(Some(e))`: An error occurred; will be yielded once then become `None`
/// - `Error(None)`: Error was already yielded; stream is terminated
///
/// # Examples
///
/// ```no_run
/// use hitbox_http::{BufferedBody, CollectExactResult, Remaining};
///
/// async fn example<B: hyper::body::Body + Unpin>(body: BufferedBody<B>) {
///     // After collecting 100 bytes from a larger body
///     let result = body.collect_exact(100).await;
///     match result {
///         CollectExactResult::AtLeast { buffered, remaining } => {
///             match remaining {
///                 Some(Remaining::Body(stream)) => {
///                     // More data available in stream
///                 }
///                 Some(Remaining::Error(err)) => {
///                     // Error occurred after collecting bytes
///                 }
///                 None => {
///                     // Stream ended exactly at limit
///                 }
///             }
///         }
///         CollectExactResult::Incomplete { .. } => {}
///     }
/// }
/// ```
#[pin_project(project = RemainingProj)]
#[derive(Debug)]
pub enum Remaining<B>
where
    B: HttpBody,
{
    /// The body stream continues with unconsumed data.
    Body(#[pin] B),
    /// An error occurred during consumption.
    ///
    /// The `Option` allows the error to be yielded once, then `None` on
    /// subsequent polls.
    Error(Option<B::Error>),
}

/// A partially consumed body: buffered prefix plus remaining stream.
///
/// Created when a predicate or extractor reads some bytes from a body stream
/// without consuming it entirely. Implements [`HttpBody`] to transparently
/// replay the buffered prefix followed by the remaining stream data.
///
/// # When You'll Encounter This
///
/// You typically don't create this directly. It appears inside
/// [`BufferedBody::Partial`] after operations like [`collect_exact`](BufferedBody::collect_exact).
///
/// # Invariants
///
/// - The prefix contains bytes already read from the original stream
/// - The remaining stream has not been polled since the prefix was extracted
/// - When polled as `HttpBody`, prefix bytes are yielded before remaining data
///
/// # Streaming Behavior
///
/// When polled as an [`HttpBody`]:
/// 1. Yields the buffered prefix (if any) as a single frame
/// 2. Delegates to the remaining stream, or yields the stored error
///
/// # Examples
///
/// ```no_run
/// use bytes::Bytes;
/// use hitbox_http::{BufferedBody, PartialBufferedBody, Remaining};
///
/// fn example<B: hyper::body::Body>(body: BufferedBody<B>) {
///     // Decompose a partial body
///     if let BufferedBody::Partial(partial) = body {
///         let prefix: Option<&Bytes> = partial.prefix();
///         println!("Buffered {} bytes", prefix.map(|b| b.len()).unwrap_or(0));
///
///         let (prefix, remaining) = partial.into_parts();
///         // Can now handle prefix and remaining separately
///     }
/// }
/// ```
///
/// # Performance
///
/// The prefix is yielded as a single frame, avoiding per-byte overhead.
/// The remaining stream is passed through without additional buffering.
#[pin_project]
pub struct PartialBufferedBody<B>
where
    B: HttpBody,
{
    prefix: Option<Bytes>,
    #[pin]
    remaining: Remaining<B>,
}

impl<B> PartialBufferedBody<B>
where
    B: HttpBody,
{
    /// Constructs a partial body for transparent stream replay.
    ///
    /// When this body is polled as [`HttpBody`], the prefix bytes are yielded
    /// first as a single frame, followed by the remaining stream data (or error).
    /// This enables predicates to inspect body content without losing data.
    pub fn new(prefix: Option<Bytes>, remaining: Remaining<B>) -> Self {
        Self { prefix, remaining }
    }

    /// Returns the bytes already consumed from the original stream.
    ///
    /// These bytes will be replayed before any remaining stream data when
    /// this body is polled. Returns `None` if no bytes were buffered.
    pub fn prefix(&self) -> Option<&Bytes> {
        self.prefix.as_ref()
    }

    /// Separates the buffered prefix from the remaining stream for independent handling.
    ///
    /// Use this when you need to process the prefix and remaining data differently,
    /// such as forwarding them to separate destinations.
    pub fn into_parts(self) -> (Option<Bytes>, Remaining<B>) {
        (self.prefix, self.remaining)
    }
}

impl<B: HttpBody> HttpBody for PartialBufferedBody<B> {
    type Data = Bytes;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();

        // First, yield the prefix if present
        if let Some(prefix) = this.prefix.take() {
            return Poll::Ready(Some(Ok(Frame::data(prefix))));
        }

        // Then handle the remaining body or error
        match this.remaining.project() {
            RemainingProj::Body(body) => match body.poll_frame(cx) {
                Poll::Ready(Some(Ok(frame))) => {
                    let frame = frame.map_data(|mut data| data.copy_to_bytes(data.remaining()));
                    Poll::Ready(Some(Ok(frame)))
                }
                Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            },
            RemainingProj::Error(error) => {
                if let Some(err) = error.take() {
                    Poll::Ready(Some(Err(err)))
                } else {
                    Poll::Ready(None)
                }
            }
        }
    }

    fn size_hint(&self) -> http_body::SizeHint {
        let prefix_len = self.prefix.as_ref().map(|b| b.len() as u64).unwrap_or(0);

        match &self.remaining {
            Remaining::Body(body) => {
                let hint = body.size_hint();
                let lower = hint.lower().saturating_add(prefix_len);

                // The upper bound needs careful handling:
                // If we have a prefix, it means we already consumed those bytes from the stream.
                // The body's upper hint might not have been updated (e.g., if based on Content-Length).
                // So we need to ensure: lower <= upper
                let upper = hint.upper().map(|u| {
                    // Upper should be at least lower to maintain the invariant
                    u.saturating_add(prefix_len).max(lower)
                });

                let mut result = http_body::SizeHint::new();
                result.set_lower(lower);
                if let Some(u) = upper {
                    result.set_upper(u);
                }
                result
            }
            Remaining::Error(_) => http_body::SizeHint::with_exact(prefix_len),
        }
    }

    fn is_end_stream(&self) -> bool {
        if self.prefix.is_some() {
            return false;
        }

        match &self.remaining {
            Remaining::Body(body) => body.is_end_stream(),
            Remaining::Error(err) => err.is_none(),
        }
    }
}

/// A body wrapper that represents different consumption states.
///
/// This enum allows predicates to partially consume request or response bodies
/// without losing data. The complete body (including any buffered prefix) is
/// forwarded to upstream services.
///
/// # States
///
/// - [`Complete`](Self::Complete): Body fully buffered in memory
/// - [`Partial`](Self::Partial): Prefix buffered, remaining stream preserved
/// - [`Passthrough`](Self::Passthrough): Untouched, zero overhead
///
/// # Examples
///
/// Creating a passthrough body for a new request:
///
/// ```
/// use bytes::Bytes;
/// use http_body_util::Empty;
/// use hitbox_http::BufferedBody;
///
/// let body: BufferedBody<Empty<Bytes>> = BufferedBody::Passthrough(Empty::new());
/// ```
///
/// Creating a complete body from cached data:
///
/// ```
/// use bytes::Bytes;
/// use http_body_util::Empty;
/// use hitbox_http::BufferedBody;
///
/// let cached_data = Bytes::from_static(b"{\"id\": 42}");
/// let body: BufferedBody<Empty<Bytes>> = BufferedBody::Complete(Some(cached_data));
/// ```
///
/// # State Transitions
///
/// ```text
/// Passthrough ──collect_exact()──► Partial (if stream continues)
///      │                               │
///      │                               ▼
///      └──────collect()──────────► Complete
/// ```
#[pin_project(project = BufferedBodyProj)]
pub enum BufferedBody<B>
where
    B: HttpBody,
{
    /// Body was fully read and buffered (within size limits).
    ///
    /// The `Option` is used to yield the data once, then return `None` on subsequent polls.
    Complete(Option<Bytes>),

    /// Body was partially read - contains buffered prefix and remaining stream.
    ///
    /// The `PartialBufferedBody` handles streaming of both the prefix and remaining data.
    Partial(#[pin] PartialBufferedBody<B>),

    /// Body was passed through without reading (untouched).
    ///
    /// The body is forwarded directly to upstream without any buffering.
    Passthrough(#[pin] B),
}

impl<B> HttpBody for BufferedBody<B>
where
    B: HttpBody,
{
    type Data = Bytes;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            BufferedBodyProj::Complete(data) => {
                if let Some(bytes) = data.take() {
                    Poll::Ready(Some(Ok(Frame::data(bytes))))
                } else {
                    Poll::Ready(None)
                }
            }

            BufferedBodyProj::Partial(partial) => {
                // Delegate to PartialBody's HttpBody implementation
                partial.poll_frame(cx)
            }

            BufferedBodyProj::Passthrough(body) => {
                // Delegate to the inner body and convert Data type
                match body.poll_frame(cx) {
                    Poll::Ready(Some(Ok(frame))) => {
                        let frame = frame.map_data(|mut data| data.copy_to_bytes(data.remaining()));
                        Poll::Ready(Some(Ok(frame)))
                    }
                    Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }

    fn size_hint(&self) -> http_body::SizeHint {
        match self {
            BufferedBody::Complete(Some(bytes)) => {
                let len = bytes.len() as u64;
                http_body::SizeHint::with_exact(len)
            }
            BufferedBody::Complete(None) => http_body::SizeHint::with_exact(0),

            BufferedBody::Partial(partial) => partial.size_hint(),

            BufferedBody::Passthrough(body) => body.size_hint(),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            BufferedBody::Complete(None) => true,
            BufferedBody::Complete(Some(_)) => false,

            BufferedBody::Partial(partial) => partial.is_end_stream(),

            BufferedBody::Passthrough(body) => body.is_end_stream(),
        }
    }
}

/// Result of attempting to collect at least N bytes from a body.
///
/// Returned by [`BufferedBody::collect_exact`] to indicate whether the
/// requested number of bytes was successfully read from the stream.
///
/// # When to Use
///
/// Use this when you need to inspect a fixed-size prefix of a body without
/// consuming the entire stream:
/// - Checking magic bytes for file type detection
/// - Reading protocol headers
/// - Validating body format before full processing
///
/// # Invariants
///
/// - `AtLeast`: `buffered.len() >= requested_bytes`
/// - `Incomplete`: `buffered.len() < requested_bytes` (stream ended or error)
/// - The buffered data may exceed the requested size if a frame boundary
///   didn't align exactly
///
/// # Examples
///
/// ```no_run
/// use hitbox_http::{BufferedBody, CollectExactResult};
///
/// async fn example<B: hyper::body::Body + Unpin>(body: BufferedBody<B>) {
///     // Check if body starts with JSON array
///     let result = body.collect_exact(1).await;
///     match result {
///         CollectExactResult::AtLeast { ref buffered, .. } => {
///             if buffered.starts_with(b"[") {
///                 // It's a JSON array, reconstruct body for further processing
///                 let body = result.into_buffered_body();
///             }
///         }
///         CollectExactResult::Incomplete { buffered, error } => {
///             // Body was empty or error occurred
///         }
///     }
/// }
/// ```
#[derive(Debug)]
pub enum CollectExactResult<B: HttpBody> {
    /// Successfully collected at least the requested number of bytes.
    ///
    /// The buffered bytes contains at least the requested amount (possibly more
    /// if a frame was consumed). The remaining field contains either:
    /// - `Some(Remaining::Body(stream))` - more data to stream
    /// - `Some(Remaining::Error(err))` - error occurred after collecting enough bytes
    /// - `None` - stream ended cleanly
    AtLeast {
        /// The bytes successfully read from the stream (at least `limit_bytes`).
        buffered: Bytes,
        /// The remaining stream data, if any.
        remaining: Option<Remaining<B>>,
    },

    /// Failed to collect the requested bytes.
    ///
    /// This occurs when either:
    /// - The body stream ended before reaching the requested number of bytes (error is None)
    /// - An error occurred while reading the stream (error is Some)
    ///
    /// The buffered field contains any bytes successfully read before the failure.
    Incomplete {
        /// Bytes read before the stream ended or error occurred.
        buffered: Option<Bytes>,
        /// The error that occurred, if any.
        error: Option<B::Error>,
    },
}

impl<B: HttpBody> CollectExactResult<B> {
    /// Converts the result into a [`BufferedBody`], using the buffered data as prefix.
    ///
    /// This reconstructs the body:
    /// - `AtLeast { buffered, remaining }` → `BufferedBody::Partial` with buffered as prefix and remaining, or `BufferedBody::Complete` if no remaining
    /// - `Incomplete { buffered, error }` → `BufferedBody::Partial` with error, or `BufferedBody::Complete` if no error
    pub fn into_buffered_body(self) -> BufferedBody<B> {
        match self {
            CollectExactResult::AtLeast {
                buffered,
                remaining,
            } => match remaining {
                Some(rem) => BufferedBody::Partial(PartialBufferedBody::new(Some(buffered), rem)),
                None => BufferedBody::Complete(Some(buffered)),
            },
            CollectExactResult::Incomplete { buffered, error } => match error {
                Some(err) => BufferedBody::Partial(PartialBufferedBody::new(
                    buffered,
                    Remaining::Error(Some(err)),
                )),
                None => BufferedBody::Complete(buffered),
            },
        }
    }
}

/// Helper function to combine an optional prefix with new data.
///
/// This is used when buffering partial bodies - we may have already consumed
/// a prefix from the stream, and now need to combine it with newly read data.
fn combine_bytes(prefix: Option<Bytes>, data: Bytes) -> Bytes {
    match prefix {
        Some(prefix_bytes) if !data.is_empty() => {
            let mut buf = BytesMut::from(prefix_bytes.as_ref());
            buf.extend_from_slice(&data);
            buf.freeze()
        }
        Some(prefix_bytes) => prefix_bytes,
        None => data,
    }
}

/// Internal result type for the low-level stream collection function.
impl<B> BufferedBody<B>
where
    B: HttpBody,
{
    /// Collects the entire body into memory.
    ///
    /// Consumes all remaining bytes from the stream and returns them as a
    /// contiguous `Bytes` buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use hitbox_http::BufferedBody;
    ///
    /// async fn example<B: hyper::body::Body>(body: BufferedBody<B>)
    /// where
    ///     B::Data: Send,
    /// {
    ///     match body.collect().await {
    ///         Ok(bytes) => println!("Collected {} bytes", bytes.len()),
    ///         Err(error_body) => {
    ///             // Error occurred, but we still have the body for forwarding
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err(BufferedBody::Partial(...))` if the underlying stream
    /// yields an error. The error is preserved in the returned body so it
    /// can be forwarded to upstream services.
    ///
    /// # Performance
    ///
    /// Allocates a buffer to hold the entire body. For large bodies, consider:
    /// - Using [`collect_exact`](Self::collect_exact) to read only a prefix
    /// - Streaming the body directly without buffering
    ///
    /// # Caveats
    ///
    /// This method blocks until the entire body is received. For very large
    /// bodies or slow streams, this may take significant time and memory.
    pub async fn collect(self) -> Result<Bytes, Self>
    where
        B::Data: Send,
    {
        use http_body_util::BodyExt;

        match self {
            // Already complete, extract bytes
            BufferedBody::Complete(Some(bytes)) => Ok(bytes),
            BufferedBody::Complete(None) => Ok(Bytes::new()),

            // Passthrough - need to collect
            BufferedBody::Passthrough(body) => match body.collect().await {
                Ok(collected) => Ok(collected.to_bytes()),
                Err(err) => Err(BufferedBody::Partial(PartialBufferedBody::new(
                    None,
                    Remaining::Error(Some(err)),
                ))),
            },

            // Partial - delegate to PartialBody which implements HttpBody
            BufferedBody::Partial(partial) => {
                let (prefix, remaining) = partial.into_parts();
                match remaining {
                    Remaining::Body(body) => match body.collect().await {
                        Ok(collected) => {
                            if let Some(prefix_bytes) = prefix {
                                let mut combined = BytesMut::from(prefix_bytes.as_ref());
                                combined.extend_from_slice(&collected.to_bytes());
                                Ok(combined.freeze())
                            } else {
                                Ok(collected.to_bytes())
                            }
                        }
                        Err(err) => Err(BufferedBody::Partial(PartialBufferedBody::new(
                            prefix,
                            Remaining::Error(Some(err)),
                        ))),
                    },
                    Remaining::Error(err) => Err(BufferedBody::Partial(PartialBufferedBody::new(
                        prefix,
                        Remaining::Error(err),
                    ))),
                }
            }
        }
    }

    /// Collects at least `limit_bytes` from the body, preserving the rest.
    ///
    /// Reads bytes from the stream until at least `limit_bytes` are buffered,
    /// then returns both the buffered prefix and the remaining stream. This
    /// enables inspecting a body prefix without consuming the entire stream.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use hitbox_http::{BufferedBody, CollectExactResult};
    ///
    /// async fn check_json_array<B: hyper::body::Body + Unpin>(
    ///     body: BufferedBody<B>,
    /// ) -> bool {
    ///     match body.collect_exact(1).await {
    ///         CollectExactResult::AtLeast { buffered, .. } => {
    ///             buffered.starts_with(b"[")
    ///         }
    ///         CollectExactResult::Incomplete { .. } => false,
    ///     }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// - [`AtLeast`](CollectExactResult::AtLeast): Collected `>= limit_bytes`; remaining stream preserved
    /// - [`Incomplete`](CollectExactResult::Incomplete): Stream ended or error before reaching limit
    ///
    /// # Errors
    ///
    /// Stream errors are captured in [`CollectExactResult::Incomplete`] with the
    /// error in the `error` field. Any bytes read before the error are preserved
    /// in `buffered`.
    ///
    /// # Performance
    ///
    /// Only allocates for the prefix buffer (up to `limit_bytes` plus one frame).
    /// The remaining stream is preserved without additional buffering.
    ///
    /// # Use Cases
    ///
    /// - Checking magic bytes for file type detection
    /// - Reading fixed-size protocol headers
    /// - Validating body format before full processing
    /// - JQ/regex predicates that need body content
    pub async fn collect_exact(self, limit_bytes: usize) -> CollectExactResult<B>
    where
        B: Unpin,
    {
        match self {
            // Already complete - check if we have enough bytes
            BufferedBody::Complete(Some(data)) => {
                if data.len() >= limit_bytes {
                    // Have at least limit_bytes, stream ended cleanly
                    CollectExactResult::AtLeast {
                        buffered: data,
                        remaining: None,
                    }
                } else {
                    // Not enough bytes
                    CollectExactResult::Incomplete {
                        buffered: Some(data),
                        error: None,
                    }
                }
            }
            BufferedBody::Complete(None) => {
                // Empty body
                CollectExactResult::Incomplete {
                    buffered: None,
                    error: None,
                }
            }

            // Partial - combine prefix with remaining stream
            BufferedBody::Partial(partial) => {
                let (prefix, remaining) = partial.into_parts();

                match prefix {
                    Some(buffered) if buffered.len() >= limit_bytes => {
                        // Prefix already has enough bytes - preserve the remaining state
                        CollectExactResult::AtLeast {
                            buffered,
                            remaining: Some(remaining),
                        }
                    }
                    prefix => {
                        // Need to read more from remaining stream
                        let prefix_len = prefix.as_ref().map(|p| p.len()).unwrap_or(0);
                        match remaining {
                            Remaining::Body(stream) => {
                                // Read more bytes from stream
                                let needed = limit_bytes - prefix_len;
                                let result = collect_exact_from_stream(stream, needed).await;
                                match result {
                                    CollectExactResult::AtLeast {
                                        buffered: new_bytes,
                                        remaining,
                                    } => {
                                        let combined = combine_bytes(prefix, new_bytes);
                                        CollectExactResult::AtLeast {
                                            buffered: combined,
                                            remaining,
                                        }
                                    }
                                    CollectExactResult::Incomplete {
                                        buffered: new_bytes,
                                        error,
                                    } => {
                                        let combined = if let Some(new) = new_bytes {
                                            Some(combine_bytes(prefix, new))
                                        } else {
                                            prefix
                                        };
                                        CollectExactResult::Incomplete {
                                            buffered: combined,
                                            error,
                                        }
                                    }
                                }
                            }
                            Remaining::Error(error) => {
                                // Already have an error, can't read more
                                CollectExactResult::Incomplete {
                                    buffered: prefix,
                                    error,
                                }
                            }
                        }
                    }
                }
            }

            // Passthrough - read from stream
            BufferedBody::Passthrough(stream) => {
                collect_exact_from_stream(stream, limit_bytes).await
            }
        }
    }
}

/// Helper function to collect exactly N bytes from a stream.
async fn collect_exact_from_stream<B>(mut stream: B, limit_bytes: usize) -> CollectExactResult<B>
where
    B: HttpBody + Unpin,
{
    use http_body_util::BodyExt;

    let mut buffer = BytesMut::new();

    // Read until we have at least limit_bytes
    while buffer.len() < limit_bytes {
        match stream.frame().await {
            Some(Ok(frame)) => {
                if let Ok(mut data) = frame.into_data() {
                    buffer.extend_from_slice(&data.copy_to_bytes(data.remaining()));
                }
            }
            Some(Err(error)) => {
                // Error while reading
                return CollectExactResult::Incomplete {
                    buffered: if buffer.is_empty() {
                        None
                    } else {
                        Some(buffer.freeze())
                    },
                    error: Some(error),
                };
            }
            None => {
                // Stream ended before we got limit_bytes
                return CollectExactResult::Incomplete {
                    buffered: if buffer.is_empty() {
                        None
                    } else {
                        Some(buffer.freeze())
                    },
                    error: None,
                };
            }
        }
    }

    // We have at least limit_bytes
    // Return the buffered data and the remaining stream
    CollectExactResult::AtLeast {
        buffered: buffer.freeze(),
        remaining: Some(Remaining::Body(stream)),
    }
}

impl<B> fmt::Debug for BufferedBody<B>
where
    B: HttpBody,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufferedBody::Complete(Some(bytes)) => f
                .debug_tuple("Complete")
                .field(&format!("{} bytes", bytes.len()))
                .finish(),
            BufferedBody::Complete(None) => f.debug_tuple("Complete").field(&"consumed").finish(),
            BufferedBody::Partial(partial) => {
                let prefix_len = partial.prefix().map(|b| b.len()).unwrap_or(0);
                f.debug_struct("Partial")
                    .field("prefix_len", &prefix_len)
                    .field("remaining", &"...")
                    .finish()
            }
            BufferedBody::Passthrough(_) => f.debug_tuple("Passthrough").field(&"...").finish(),
        }
    }
}
