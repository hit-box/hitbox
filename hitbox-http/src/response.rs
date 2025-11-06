use std::fmt::Debug;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use hitbox::{
    CachePolicy, CacheValue, CacheableResponse, EntityPolicyConfig, predicate::PredicateResult,
};
use http::{HeaderMap, Response, response::Parts};
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::{body::FromBytes, predicates::body::HttpPartition};

#[derive(Debug)]
pub enum ResponseBody<ResBody> {
    Pending(ResBody),
    Complete(Bytes),
}

impl<ResBody> ResponseBody<ResBody>
where
    ResBody: FromBytes,
{
    pub fn into_inner_body(self) -> ResBody {
        match self {
            ResponseBody::Pending(body) => body,
            ResponseBody::Complete(body) => ResBody::from_bytes(body),
        }
    }
}

#[derive(Debug)]
pub struct CacheableHttpResponse<ResBody> {
    pub parts: Parts,
    pub body: ResponseBody<ResBody>,
}

impl<ResBody> CacheableHttpResponse<ResBody>
where
    ResBody: FromBytes,
{
    pub fn from_response(response: Response<ResBody>) -> Self {
        let (parts, body) = response.into_parts();
        CacheableHttpResponse {
            parts,
            body: ResponseBody::Pending(body),
        }
    }

    pub fn into_response(self) -> Response<ResBody> {
        match self.body {
            ResponseBody::Pending(body) => Response::from_parts(self.parts, body),
            ResponseBody::Complete(body) => {
                Response::from_parts(self.parts, ResBody::from_bytes(body))
            }
        }
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
    ResBody: HttpBody + FromBytes + Send + 'static,
    // debug bounds
    ResBody::Error: Debug,
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
        use http_body_util::BodyExt;
        let body = self
            .body
            .into_inner_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();

        // We can store the HeaderMap directly, including pseudo-headers
        // HeaderMap is designed to handle pseudo-headers and http-serde will serialize them correctly
        CachePolicy::Cacheable(SerializableHttpResponse {
            status: self.parts.status.as_u16(),
            version: format!("{:?}", self.parts.version),
            body,
            headers: self.parts.headers,
        })
    }

    async fn from_cached(cached: Self::Cached) -> Self {
        let body = ResBody::from_bytes(Bytes::from(cached.body));
        let mut response = Response::builder()
            .status(cached.status)
            .body(body)
            .unwrap();

        // Replace the headers with the cached HeaderMap
        *response.headers_mut() = cached.headers;

        CacheableHttpResponse::from_response(response)
    }
}

impl<ResBody> HttpPartition for CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody + FromBytes,
{
    type Body = ResBody;
    type Parts = Parts;

    fn into_parts_and_body(self) -> (Self::Parts, Self::Body) {
        self.into_response().into_parts()
    }

    fn from_parts_and_body(parts: Self::Parts, body: Self::Body) -> Self {
        Self::from_response(Response::from_parts(parts, body))
    }
}
