use axum::body::Body;
use bytes::{Buf, Bytes, BytesMut};
use http_body_util::{BodyExt, Empty, Full, combinators::UnsyncBoxBody};

pub trait FromBytes {
    fn from_bytes(bytes: Bytes) -> Self;
}

/// Trait for constructing a body from chunks that may contain errors.
/// This allows reconstructing the body after collection, preserving any
/// network errors that occurred during streaming.
pub trait FromChunks<E> {
    fn from_chunks(chunks: Vec<Result<Bytes, E>>) -> Self;
}

impl FromBytes for Body {
    fn from_bytes(bytes: Bytes) -> Self {
        Body::from(bytes)
    }
}

impl<E> FromChunks<E> for Body {
    fn from_chunks(chunks: Vec<Result<Bytes, E>>) -> Self {
        // Concatenate all successful chunks
        let payload: Bytes = chunks
            .into_iter()
            .filter_map(|chunk| chunk.ok())
            .fold(BytesMut::new(), |mut acc, bytes| {
                acc.extend_from_slice(&bytes);
                acc
            })
            .freeze();
        Body::from(payload)
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

impl<D, E> FromChunks<E> for Full<D>
where
    D: From<Bytes> + Buf + Send + 'static,
{
    fn from_chunks(chunks: Vec<Result<Bytes, E>>) -> Self {
        // Concatenate all successful chunks
        let payload: Bytes = chunks
            .into_iter()
            .filter_map(|chunk| chunk.ok())
            .fold(BytesMut::new(), |mut acc, bytes| {
                acc.extend_from_slice(&bytes);
                acc
            })
            .freeze();
        Full::new(D::from(payload))
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
