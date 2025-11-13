use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use hitbox::{
    CachePolicy, CacheValue, CacheableResponse, EntityPolicyConfig, predicate::PredicateResult,
};
use http::{HeaderMap, Response, response::Parts};
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::body::BufferedBody;

#[derive(Debug)]
pub struct CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody,
{
    pub parts: Parts,
    pub body: BufferedBody<ResBody>,
}

impl<ResBody> CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody,
{
    pub fn from_response(response: Response<BufferedBody<ResBody>>) -> Self {
        let (parts, body) = response.into_parts();
        CacheableHttpResponse { parts, body }
    }

    pub fn into_response(self) -> Response<BufferedBody<ResBody>> {
        Response::from_parts(self.parts, self.body)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableHttpResponse {
    status: u16,
    version: String,
    #[serde(with = "serde_bytes")]
    body: Vec<u8>,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
}

#[async_trait]
impl<ResBody> CacheableResponse for CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody + Send + 'static,
    // debug bounds
    ResBody::Error: Debug + Send,
    ResBody::Data: Send,
{
    type Cached = SerializableHttpResponse;
    type Subject = Self;

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

    async fn into_cached(self) -> CachePolicy<Self::Cached, Self> {
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
            status: self.parts.status.as_u16(),
            version: format!("{:?}", self.parts.version),
            body: body_bytes.to_vec(),
            headers: self.parts.headers,
        })
    }

    async fn from_cached(cached: Self::Cached) -> Self {
        let body = BufferedBody::Complete(Some(Bytes::from(cached.body)));
        let mut response = Response::builder()
            .status(cached.status)
            .body(body)
            .unwrap();

        // Replace the headers with the cached HeaderMap
        *response.headers_mut() = cached.headers;

        CacheableHttpResponse::from_response(response)
    }
}
