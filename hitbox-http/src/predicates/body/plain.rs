//! Plain-text body matching operations.
//!
//! Provides [`PlainOperation`] for matching raw body bytes using equality,
//! prefix/suffix checks, substring search, and regular expressions.

use std::fmt::Debug;

use bytes::{Buf, Bytes, BytesMut};
use hitbox::predicate::PredicateResult;
use http_body_util::BodyExt;
use hyper::body::Body as HttpBody;

use crate::{BufferedBody, PartialBufferedBody, Remaining};

/// Searches for a pattern in a body stream without collecting the entire body.
///
/// Optimizes the search by stopping early when the pattern is found and handling
/// patterns that span chunk boundaries.
async fn streaming_search<B>(body: BufferedBody<B>, pattern: &[u8]) -> (bool, BufferedBody<B>)
where
    B: HttpBody + Unpin,
    B::Error: Debug,
    B::Data: Send,
{
    if pattern.is_empty() {
        return (true, body);
    }

    match body {
        // Already complete - just search
        BufferedBody::Complete(Some(bytes)) => {
            let found = bytes.windows(pattern.len()).any(|w| w == pattern);
            (found, BufferedBody::Complete(Some(bytes)))
        }
        BufferedBody::Complete(None) => (false, BufferedBody::Complete(None)),

        // Partial - extract parts and search through prefix + remaining
        BufferedBody::Partial(partial) => {
            let (prefix, remaining) = partial.into_parts();
            match remaining {
                Remaining::Body(body) => streaming_search_body(prefix, body, pattern).await,
                Remaining::Error(error) => {
                    // Already have an error, just search in the prefix we have
                    let found = prefix
                        .as_ref()
                        .map(|b| b.windows(pattern.len()).any(|w| w == pattern))
                        .unwrap_or(false);
                    (
                        found,
                        BufferedBody::Partial(PartialBufferedBody::new(
                            prefix,
                            Remaining::Error(error),
                        )),
                    )
                }
            }
        }

        // Passthrough - stream through it
        BufferedBody::Passthrough(stream) => streaming_search_body(None, stream, pattern).await,
    }
}

/// Helper function that performs streaming search on any Body implementation.
/// Optionally starts with an initial prefix buffer.
async fn streaming_search_body<B>(
    initial_prefix: Option<Bytes>,
    mut body: B,
    pattern: &[u8],
) -> (bool, BufferedBody<B>)
where
    B: HttpBody + Unpin,
    B::Error: Debug,
    B::Data: Send,
{
    let mut buffer = BytesMut::new();

    // Initialize with prefix if provided
    if let Some(prefix_bytes) = initial_prefix {
        buffer.extend_from_slice(&prefix_bytes);
    }

    // Keep last (pattern.len() - 1) bytes to handle pattern spanning chunks
    let overlap_size = pattern.len().saturating_sub(1);

    loop {
        match body.frame().await {
            Some(Ok(frame)) => {
                if let Ok(mut data) = frame.into_data() {
                    // Search in: [overlap from previous] + [current chunk]
                    let search_start = buffer.len().saturating_sub(overlap_size);
                    buffer.extend_from_slice(&data.copy_to_bytes(data.remaining()));

                    // Search in the new region (overlap + new data)
                    if buffer[search_start..]
                        .windows(pattern.len())
                        .any(|w| w == pattern)
                    {
                        // Found! Return complete body with all buffered data
                        return (true, BufferedBody::Complete(Some(buffer.freeze())));
                    }
                }
            }
            Some(Err(error)) => {
                // Error occurred - save buffered data + error in Partial body
                let buffered = if buffer.is_empty() {
                    None
                } else {
                    Some(buffer.freeze())
                };

                // Check if pattern was in buffered data before error
                let found = buffered
                    .as_ref()
                    .map(|b| b.windows(pattern.len()).any(|w| w == pattern))
                    .unwrap_or(false);

                let result_body = BufferedBody::Partial(PartialBufferedBody::new(
                    buffered,
                    Remaining::Error(Some(error)),
                ));
                return (found, result_body);
            }
            None => {
                // End of stream
                let combined = if buffer.is_empty() {
                    None
                } else {
                    Some(buffer.freeze())
                };

                let found = combined
                    .as_ref()
                    .map(|b| b.windows(pattern.len()).any(|w| w == pattern))
                    .unwrap_or(false);

                return (found, BufferedBody::Complete(combined));
            }
        }
    }
}

/// Operations for matching raw body bytes.
///
/// Each operation checks the body content against a pattern and returns
/// [`Cacheable`](PredicateResult::Cacheable) on match or
/// [`NonCacheable`](PredicateResult::NonCacheable) otherwise.
///
/// # Performance
///
/// - [`Starts`](Self::Starts): Reads only the prefix bytes needed for comparison
/// - [`Contains`](Self::Contains): Uses streaming search, stops early on match
/// - [`Eq`](Self::Eq), [`Ends`](Self::Ends), [`RegExp`](Self::RegExp): Collect the entire body
///
/// # Caveats
///
/// All operations consume the body stream. The body is returned as
/// [`BufferedBody::Complete`] or [`BufferedBody::Partial`] after checking.
#[derive(Debug)]
pub enum PlainOperation {
    /// Use when the entire body must match exactly.
    ///
    /// Best for known static responses or signatures.
    Eq(Bytes),
    /// Use when a marker or pattern appears anywhere in the body.
    ///
    /// Best for success markers, error messages, or content indicators.
    /// Uses streaming search for efficiency.
    Contains(Bytes),
    /// Use when the body should begin with a specific prefix.
    ///
    /// Best for magic numbers, file signatures, or protocol headers.
    /// Only reads the prefix bytes needed.
    Starts(Bytes),
    /// Use when the body should end with a specific suffix.
    ///
    /// Best for file extensions embedded in content or trailing markers.
    /// Requires reading the entire body.
    Ends(Bytes),
    /// Use when matching complex patterns in body content.
    ///
    /// Best for structured text, log formats, or flexible content matching.
    /// Requires reading the entire body.
    RegExp(regex::bytes::Regex),
}

impl PlainOperation {
    /// Checks if this operation matches the body.
    ///
    /// Returns [`Cacheable`](PredicateResult::Cacheable) when the body matches,
    /// [`NonCacheable`](PredicateResult::NonCacheable) otherwise.
    ///
    /// The returned body is always in a buffered state suitable for further
    /// processing or caching.
    pub async fn check<B>(&self, body: BufferedBody<B>) -> PredicateResult<BufferedBody<B>>
    where
        B: HttpBody + Unpin,
        B::Data: Send,
        B::Error: Debug,
    {
        match self {
            PlainOperation::Starts(prefix) => {
                // Empty prefix always matches
                if prefix.is_empty() {
                    return PredicateResult::Cacheable(body);
                }

                // Use collect_exact to read exactly prefix.len() bytes
                use crate::CollectExactResult;

                let result = body.collect_exact(prefix.len()).await;

                // Check if body starts with prefix
                let matches = match &result {
                    CollectExactResult::AtLeast { buffered, .. } => buffered.starts_with(prefix),
                    CollectExactResult::Incomplete { .. } => false, // Not enough bytes
                };

                // Reconstruct body
                let result_body = result.into_buffered_body();

                if matches {
                    PredicateResult::Cacheable(result_body)
                } else {
                    PredicateResult::NonCacheable(result_body)
                }
            }

            PlainOperation::Eq(expected) => body
                .collect()
                .await
                .map(|body_bytes| {
                    let matches = body_bytes.as_ref() == expected.as_ref();
                    let result_body = BufferedBody::Complete(Some(body_bytes));
                    if matches {
                        PredicateResult::Cacheable(result_body)
                    } else {
                        PredicateResult::NonCacheable(result_body)
                    }
                })
                .unwrap_or_else(PredicateResult::NonCacheable),

            PlainOperation::Contains(sequence) => {
                let (found, result_body) = streaming_search(body, sequence.as_ref()).await;
                if found {
                    PredicateResult::Cacheable(result_body)
                } else {
                    PredicateResult::NonCacheable(result_body)
                }
            }

            PlainOperation::Ends(suffix) => body
                .collect()
                .await
                .map(|body_bytes| {
                    let matches = body_bytes.ends_with(suffix);
                    let result_body = BufferedBody::Complete(Some(body_bytes));
                    if matches {
                        PredicateResult::Cacheable(result_body)
                    } else {
                        PredicateResult::NonCacheable(result_body)
                    }
                })
                .unwrap_or_else(PredicateResult::NonCacheable),

            PlainOperation::RegExp(regex) => body
                .collect()
                .await
                .map(|body_bytes| {
                    let matches = regex.is_match(body_bytes.as_ref());
                    let result_body = BufferedBody::Complete(Some(body_bytes));
                    if matches {
                        PredicateResult::Cacheable(result_body)
                    } else {
                        PredicateResult::NonCacheable(result_body)
                    }
                })
                .unwrap_or_else(PredicateResult::NonCacheable),
        }
    }
}
