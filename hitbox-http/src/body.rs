use axum::body::Body;
use bytes::{Buf, Bytes};
use http_body::{Body as HttpBody, Frame};
use http_body_util::{BodyExt, Empty, Full, combinators::UnsyncBoxBody};
use pin_project::pin_project;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait FromBytes {
    fn from_bytes(bytes: Bytes) -> Self;
}

impl FromBytes for Body {
    fn from_bytes(bytes: Bytes) -> Self {
        Body::from(bytes)
    }
}

// impl<D> FromBytes for Collected<D> {
//     fn from_bytes(bytes: Bytes) -> Self {
//         Collected::from(bytes)
//     }
// }

impl<D, E> FromBytes for UnsyncBoxBody<D, E>
where
    D: From<Bytes> + Buf + Send + 'static,
    E: 'static,
{
    fn from_bytes(bytes: Bytes) -> Self {
        UnsyncBoxBody::new(Full::new(D::from(bytes)).map_err(|_| unreachable!()))
    }
}

impl<D> FromBytes for Full<D>
where
    D: From<Bytes> + Buf + Send + 'static,
{
    fn from_bytes(bytes: Bytes) -> Self {
        Full::new(D::from(bytes))
    }
}

impl<D> FromBytes for Empty<D>
where
    D: From<Bytes> + Buf + Send + 'static,
{
    fn from_bytes(_bytes: Bytes) -> Self {
        Empty::new()
    }
}

impl FromBytes for String {
    fn from_bytes(bytes: Bytes) -> Self {
        String::from_utf8_lossy(&bytes).to_string()
    }
}

/// Internal enum to represent the remaining body state after partial consumption.
#[pin_project(project = RemainingProj)]
enum Remaining<B>
where
    B: HttpBody,
{
    /// The body stream continues
    Body(#[pin] B),
    /// An error was encountered during consumption - yield once then end stream
    Error(Option<B::Error>),
}

/// A body wrapper that represents different consumption states.
///
/// This enum allows predicates to partially consume request or response bodies
/// without losing data. The complete body (including any buffered prefix) is
/// forwarded to upstream services.
///
/// # Variants
///
/// - [`Complete`](BufferedBody::Complete): Body was fully read and buffered (within size limits)
/// - [`Partial`](BufferedBody::Partial): Body was partially read - has buffered prefix plus
///   remaining stream or error
/// - [`Passthrough`](BufferedBody::Passthrough): Body was not read at all (untouched)
#[pin_project(project = BufferedBodyProj)]
pub enum BufferedBody<B>
where
    B: HttpBody,
{
    /// Body was fully read and buffered (within size limits).
    ///
    /// The `Option` is used to yield the data once, then return `None` on subsequent polls.
    Complete(Option<Bytes>),

    /// Body was partially read - has buffered prefix and remaining stream or error.
    ///
    /// The buffered `prefix` (if any) is yielded first, then either:
    /// - An error is yielded (if present), or
    /// - The remaining body stream continues
    Partial {
        /// Buffered data read during predicate evaluation.
        /// Yielded once, then becomes `None`.
        prefix: Option<Bytes>,
        /// Either the remaining body stream or an error encountered during reading.
        #[pin]
        remaining: Remaining<B>,
    },

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

            BufferedBodyProj::Partial { prefix, remaining } => {
                // First, yield any buffered prefix
                if let Some(data) = prefix.take() {
                    return Poll::Ready(Some(Ok(Frame::data(data))));
                }

                // Then handle the remaining body or error
                match remaining.project() {
                    RemainingProj::Body(body) => {
                        // Continue polling the remaining body stream and convert Data type
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
                    RemainingProj::Error(error) => {
                        // Yield the error once, then end the stream
                        if let Some(err) = error.take() {
                            Poll::Ready(Some(Err(err)))
                        } else {
                            Poll::Ready(None)
                        }
                    }
                }
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

            BufferedBody::Partial { prefix, remaining } => {
                let prefix_len = prefix.as_ref().map(|b| b.len() as u64).unwrap_or(0);

                match remaining {
                    Remaining::Body(body) => {
                        let mut hint = body.size_hint();
                        // Add prefix length to both lower and upper bounds
                        hint.set_lower(hint.lower().saturating_add(prefix_len));
                        if let Some(upper) = hint.upper() {
                            hint.set_upper(upper.saturating_add(prefix_len));
                        }
                        hint
                    }
                    Remaining::Error(_) => {
                        // Only the prefix will be yielded (error doesn't contribute to size)
                        http_body::SizeHint::with_exact(prefix_len)
                    }
                }
            }

            BufferedBody::Passthrough(body) => body.size_hint(),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            BufferedBody::Complete(None) => true,
            BufferedBody::Complete(Some(_)) => false,

            BufferedBody::Partial { prefix, remaining } => {
                if prefix.is_some() {
                    return false;
                }

                match remaining {
                    Remaining::Body(body) => body.is_end_stream(),
                    Remaining::Error(err) => err.is_none(),
                }
            }

            BufferedBody::Passthrough(body) => body.is_end_stream(),
        }
    }
}

impl<B> FromBytes for BufferedBody<B>
where
    B: HttpBody,
{
    fn from_bytes(bytes: Bytes) -> Self {
        BufferedBody::Complete(Some(bytes))
    }
}

impl<B> fmt::Debug for BufferedBody<B>
where
    B: HttpBody,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufferedBody::Complete(Some(bytes)) => {
                f.debug_tuple("Complete").field(&format!("{} bytes", bytes.len())).finish()
            }
            BufferedBody::Complete(None) => {
                f.debug_tuple("Complete").field(&"consumed").finish()
            }
            BufferedBody::Partial { prefix, .. } => {
                let prefix_len = prefix.as_ref().map(|b| b.len()).unwrap_or(0);
                f.debug_struct("Partial")
                    .field("prefix_len", &prefix_len)
                    .field("remaining", &"...")
                    .finish()
            }
            BufferedBody::Passthrough(_) => {
                f.debug_tuple("Passthrough").field(&"...").finish()
            }
        }
    }
}
