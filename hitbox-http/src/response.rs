use std::{collections::HashMap, fmt::Debug};

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use hitbox::{
    predicate::PredicateResult, CachePolicy, CacheValue, CacheableResponse, EntityPolicyConfig,
};
use http::{response::Parts, Response};
use hyper::body::{to_bytes, HttpBody};
use serde::{Deserialize, Serialize};

use crate::body::FromBytes;

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
    headers: HashMap<String, String>,
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
        CachePolicy::Cacheable(SerializableHttpResponse {
            status: 200,
            version: "HTTP/1.1".to_owned(),
            body: to_bytes(self.body.into_inner_body())
                .await
                .unwrap()
                .to_vec(),
            headers: self
                .parts
                .headers
                .into_iter()
                .map(|(h, v)| (h.unwrap().to_string(), v.to_str().unwrap().to_string()))
                .collect(),
        })
    }

    async fn from_cached(cached: Self::Cached) -> Self {
        let mut inner = Response::builder();
        for (key, value) in cached.headers.into_iter() {
            inner = inner.header(key, value)
        }
        let body = ResBody::from_bytes(Bytes::from(cached.body));
        let inner = inner.status(cached.status).body(body).unwrap();
        CacheableHttpResponse::from_response(inner)
    }
}
