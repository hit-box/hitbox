use std::fmt::Debug;
use std::future::Ready;

use bytes::Bytes;
use chrono::Utc;
use futures::FutureExt;
use futures::future::BoxFuture;
use hitbox::{
    CachePolicy, CacheValue, CacheableResponse, EntityPolicyConfig, predicate::PredicateResult,
};
use http::{HeaderMap, Response, response::Parts};
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::CacheableSubject;
use crate::body::BufferedBody;
use crate::predicates::header::HasHeaders;
use crate::predicates::version::HasVersion;

/// Wraps an HTTP response for cache storage and retrieval.
///
/// This type holds the response metadata ([`Parts`]) and a [`BufferedBody`].
/// When caching, the body is collected into bytes and stored as a
/// [`SerializableHttpResponse`]. When retrieving from cache, the response
/// is reconstructed with a [`BufferedBody::Complete`] containing the cached bytes.
///
/// # Type Parameters
///
/// * `ResBody` - The HTTP response body type. Must implement [`hyper::body::Body`]
///   with `Send` bounds. Common concrete types:
///   - [`Empty<Bytes>`](http_body_util::Empty) - No body (204 responses)
///   - [`Full<Bytes>`](http_body_util::Full) - Complete body in memory
///   - `BoxBody<Bytes, E>` - Type-erased body for dynamic dispatch
///
/// # Examples
///
/// ```
/// use bytes::Bytes;
/// use http::Response;
/// use http_body_util::Empty;
/// use hitbox_http::{BufferedBody, CacheableHttpResponse};
///
/// let response = Response::builder()
///     .status(200)
///     .header("Content-Type", "application/json")
///     .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
///     .unwrap();
///
/// let cacheable = CacheableHttpResponse::from_response(response);
/// ```
///
/// # Cache Storage
///
/// When a response is cacheable, the body is fully collected and stored as
/// [`SerializableHttpResponse`]. This means:
///
/// - The entire response body must fit in memory
/// - Streaming responses are buffered before storage
/// - Body collection errors result in a `NonCacheable` policy
///
/// # Performance
///
/// Retrieving a cached response is allocation-efficient: the body bytes are
/// wrapped directly in [`BufferedBody::Complete`] without copying.
#[derive(Debug)]
pub struct CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody,
{
    /// Response metadata (status, version, headers, extensions).
    pub parts: Parts,
    /// The response body in one of three buffering states.
    pub body: BufferedBody<ResBody>,
}

impl<ResBody> CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody,
{
    /// Creates a cacheable response from an HTTP response with a buffered body.
    ///
    /// The response body must already be wrapped in a [`BufferedBody`]. Use
    /// [`BufferedBody::Passthrough`] for responses that haven't been inspected yet.
    pub fn from_response(response: Response<BufferedBody<ResBody>>) -> Self {
        let (parts, body) = response.into_parts();
        CacheableHttpResponse { parts, body }
    }

    /// Converts back into a standard HTTP response.
    ///
    /// Use this after cache policy evaluation to send the response to the client.
    pub fn into_response(self) -> Response<BufferedBody<ResBody>> {
        Response::from_parts(self.parts, self.body)
    }
}

impl<ResBody> CacheableSubject for CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody,
{
    type Body = ResBody;
    type Parts = Parts;

    fn into_parts(self) -> (Self::Parts, BufferedBody<Self::Body>) {
        (self.parts, self.body)
    }

    fn from_parts(parts: Self::Parts, body: BufferedBody<Self::Body>) -> Self {
        Self { parts, body }
    }
}

impl<ResBody> HasHeaders for CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody,
{
    fn headers(&self) -> &http::HeaderMap {
        &self.parts.headers
    }
}

impl<ResBody> HasVersion for CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody,
{
    fn http_version(&self) -> http::Version {
        self.parts.version
    }
}

#[cfg(feature = "rkyv_format")]
mod rkyv_version {
    use http::Version;
    use rkyv::{
        Place,
        rancor::Fallible,
        with::{ArchiveWith, DeserializeWith, SerializeWith},
    };

    /// Serialize HTTP version as u8 using intuitive numbering:
    /// HTTP/0.9 => 9, HTTP/1.0 => 10, HTTP/1.1 => 11, HTTP/2 => 20, HTTP/3 => 30
    pub struct VersionAsU8;

    impl ArchiveWith<Version> for VersionAsU8 {
        type Archived = rkyv::Archived<u8>;
        type Resolver = rkyv::Resolver<u8>;

        fn resolve_with(field: &Version, resolver: Self::Resolver, out: Place<Self::Archived>) {
            rkyv::Archive::resolve(&version_to_u8(*field), resolver, out);
        }
    }

    impl<S: Fallible + rkyv::ser::Writer + ?Sized> SerializeWith<Version, S> for VersionAsU8 {
        fn serialize_with(field: &Version, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
            rkyv::Serialize::serialize(&version_to_u8(*field), serializer)
        }
    }

    impl<D: Fallible + ?Sized> DeserializeWith<rkyv::Archived<u8>, Version, D> for VersionAsU8 {
        fn deserialize_with(
            field: &rkyv::Archived<u8>,
            deserializer: &mut D,
        ) -> Result<Version, D::Error> {
            let value: u8 = rkyv::Deserialize::deserialize(field, deserializer)?;
            Ok(u8_to_version(value))
        }
    }

    fn version_to_u8(version: Version) -> u8 {
        match version {
            Version::HTTP_09 => 9,
            Version::HTTP_10 => 10,
            Version::HTTP_11 => 11,
            Version::HTTP_2 => 20,
            Version::HTTP_3 => 30,
            _ => 11, // Default to HTTP/1.1
        }
    }

    fn u8_to_version(value: u8) -> Version {
        match value {
            9 => Version::HTTP_09,
            10 => Version::HTTP_10,
            11 => Version::HTTP_11,
            20 => Version::HTTP_2,
            30 => Version::HTTP_3,
            _ => Version::HTTP_11, // Default to HTTP/1.1
        }
    }
}

#[cfg(feature = "rkyv_format")]
mod rkyv_status_code {
    use http::StatusCode;
    use rkyv::{
        Place,
        rancor::Fallible,
        with::{ArchiveWith, DeserializeWith, SerializeWith},
    };

    pub struct StatusCodeAsU16;

    impl ArchiveWith<StatusCode> for StatusCodeAsU16 {
        type Archived = rkyv::Archived<u16>;
        type Resolver = rkyv::Resolver<u16>;

        fn resolve_with(field: &StatusCode, resolver: Self::Resolver, out: Place<Self::Archived>) {
            let value = field.as_u16();
            rkyv::Archive::resolve(&value, resolver, out);
        }
    }

    impl<S: Fallible + rkyv::ser::Writer + ?Sized> SerializeWith<StatusCode, S> for StatusCodeAsU16 {
        fn serialize_with(
            field: &StatusCode,
            serializer: &mut S,
        ) -> Result<Self::Resolver, S::Error> {
            rkyv::Serialize::serialize(&field.as_u16(), serializer)
        }
    }

    impl<D: Fallible + ?Sized> DeserializeWith<rkyv::Archived<u16>, StatusCode, D> for StatusCodeAsU16 {
        fn deserialize_with(
            field: &rkyv::Archived<u16>,
            deserializer: &mut D,
        ) -> Result<StatusCode, D::Error> {
            let value: u16 = rkyv::Deserialize::deserialize(field, deserializer)?;
            // StatusCode::from_u16 always succeeds for valid u16 values
            Ok(StatusCode::from_u16(value).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[cfg(feature = "rkyv_format")]
mod rkyv_header_map {
    use http::HeaderMap;
    use rkyv::{
        Place,
        rancor::Fallible,
        with::{ArchiveWith, DeserializeWith, SerializeWith},
    };

    pub struct AsHeaderVec;

    impl ArchiveWith<HeaderMap> for AsHeaderVec {
        type Archived = rkyv::Archived<Vec<(String, Vec<u8>)>>;
        type Resolver = rkyv::Resolver<Vec<(String, Vec<u8>)>>;

        fn resolve_with(field: &HeaderMap, resolver: Self::Resolver, out: Place<Self::Archived>) {
            let vec: Vec<(String, Vec<u8>)> = field
                .iter()
                .map(|(name, value)| (name.as_str().to_string(), value.as_bytes().to_vec()))
                .collect();
            rkyv::Archive::resolve(&vec, resolver, out);
        }
    }

    impl<S> SerializeWith<HeaderMap, S> for AsHeaderVec
    where
        S: Fallible + rkyv::ser::Writer + rkyv::ser::Allocator + ?Sized,
        S::Error: rkyv::rancor::Source,
    {
        fn serialize_with(
            field: &HeaderMap,
            serializer: &mut S,
        ) -> Result<Self::Resolver, S::Error> {
            let vec: Vec<(String, Vec<u8>)> = field
                .iter()
                .map(|(name, value)| (name.as_str().to_string(), value.as_bytes().to_vec()))
                .collect();
            rkyv::Serialize::serialize(&vec, serializer)
        }
    }

    impl<D> DeserializeWith<rkyv::Archived<Vec<(String, Vec<u8>)>>, HeaderMap, D> for AsHeaderVec
    where
        D: Fallible + ?Sized,
    {
        fn deserialize_with(
            field: &rkyv::Archived<Vec<(String, Vec<u8>)>>,
            _deserializer: &mut D,
        ) -> Result<HeaderMap, D::Error> {
            // Zero-copy optimization: work directly with archived data
            // instead of deserializing intermediate Vec<(String, Vec<u8>)>
            let mut map = HeaderMap::with_capacity(field.len());

            for item in field.iter() {
                // Access archived data directly without allocation
                let name_str: &str = item.0.as_str();
                let value_slice: &[u8] = item.1.as_slice();

                if let (Ok(header_name), Ok(header_value)) = (
                    http::header::HeaderName::from_bytes(name_str.as_bytes()),
                    http::header::HeaderValue::from_bytes(value_slice),
                ) {
                    map.append(header_name, header_value);
                }
            }
            Ok(map)
        }
    }
}

/// Serialized form of an HTTP response for cache storage.
///
/// This type captures all information needed to reconstruct an HTTP response:
/// status code, HTTP version, headers, and body bytes. It supports multiple
/// serialization formats through serde and optionally rkyv for zero-copy
/// deserialization.
///
/// # Serialization Formats
///
/// - **JSON/Bincode**: Standard serde serialization via [`http_serde`]
/// - **rkyv** (feature `rkyv_format`): Zero-copy deserialization for maximum
///   performance when reading from cache
///
/// # Performance
///
/// With the `rkyv_format` feature enabled:
/// - HTTP version is stored as a single byte (9, 10, 11, 20, 30)
/// - Status code is stored as a u16
/// - Headers are stored as `Vec<(String, Vec<u8>)>` for zero-copy access
///
/// This allows cache hits to avoid most allocations during deserialization.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct SerializableHttpResponse {
    #[serde(with = "http_serde::status_code")]
    #[cfg_attr(feature = "rkyv_format", rkyv(with = rkyv_status_code::StatusCodeAsU16))]
    status: http::StatusCode,
    #[serde(with = "http_serde::version")]
    #[cfg_attr(feature = "rkyv_format", rkyv(with = rkyv_version::VersionAsU8))]
    version: http::Version,
    body: Bytes,
    #[serde(with = "http_serde::header_map")]
    #[cfg_attr(feature = "rkyv_format", rkyv(with = rkyv_header_map::AsHeaderVec))]
    headers: HeaderMap,
}

impl<ResBody> CacheableResponse for CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody + Send + 'static,
    // debug bounds
    ResBody::Error: Debug + Send,
    ResBody::Data: Send,
{
    type Cached = SerializableHttpResponse;
    type Subject = Self;
    type IntoCachedFuture = BoxFuture<'static, CachePolicy<Self::Cached, Self>>;
    type FromCachedFuture = Ready<Self>;

    async fn cache_policy<P>(
        self,
        predicates: P,
        config: &EntityPolicyConfig,
    ) -> hitbox::ResponseCachePolicy<Self>
    where
        P: hitbox::Predicate<Subject = Self::Subject> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(cacheable) => match cacheable.into_cached().await {
                CachePolicy::Cacheable(res) => CachePolicy::Cacheable(CacheValue::new(
                    res,
                    config.ttl.map(|duration| Utc::now() + duration),
                    config.stale_ttl.map(|duration| Utc::now() + duration),
                )),
                CachePolicy::NonCacheable(res) => CachePolicy::NonCacheable(res),
            },
            PredicateResult::NonCacheable(res) => CachePolicy::NonCacheable(res),
        }
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        async move {
            let body_bytes = match self.body.collect().await {
                Ok(bytes) => bytes,
                Err(error_body) => {
                    // If collection fails, return NonCacheable with error body
                    return CachePolicy::NonCacheable(CacheableHttpResponse {
                        parts: self.parts,
                        body: error_body,
                    });
                }
            };

            // We can store the HeaderMap directly, including pseudo-headers
            // HeaderMap is designed to handle pseudo-headers and http-serde will serialize them correctly
            CachePolicy::Cacheable(SerializableHttpResponse {
                status: self.parts.status,
                version: self.parts.version,
                body: body_bytes,
                headers: self.parts.headers,
            })
        }
        .boxed()
    }

    fn from_cached(cached: Self::Cached) -> Self::FromCachedFuture {
        let body = BufferedBody::Complete(Some(cached.body));
        let mut response = Response::new(body);
        *response.status_mut() = cached.status;
        *response.version_mut() = cached.version;
        *response.headers_mut() = cached.headers;

        std::future::ready(CacheableHttpResponse::from_response(response))
    }
}
