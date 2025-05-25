use axum::body::Body;
use bytes::{Buf, Bytes};
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Collected, Full};

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
