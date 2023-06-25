use std::collections::HashMap;

use async_trait::async_trait;
use hitbox::cache::CacheableResponse;
use http::{response::Parts, Response};
use hyper::{body::to_bytes, Body};
use serde::{Deserialize, Serialize};

pub struct CacheableHttpResponse {
    parts: Parts,
    body: Body,
}

impl CacheableHttpResponse {
    pub fn new(response: Response<Body>) -> Self {
        let (parts, body) = response.into_parts();
        CacheableHttpResponse { parts, body }
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
impl CacheableResponse for CacheableHttpResponse {
    type Cached = SerializableHttpResponse;

    async fn into_cached(self) -> Self::Cached {
        SerializableHttpResponse {
            status: 200,
            version: "HTTP/1.1".to_owned(),
            body: to_bytes(self.body).await.unwrap().to_vec(),
            headers: self
                .parts
                .headers
                .into_iter()
                .map(|(h, v)| (h.unwrap().to_string(), v.to_str().unwrap().to_string()))
                .collect(),
        }
    }

    async fn from_cached(cached: Self::Cached) -> Self {
        let mut inner = Response::builder();
        for (key, value) in cached.headers.into_iter() {
            inner = inner.header(key, value)
        }
        let body = cached.body.into();
        let inner = inner.status(cached.status).body(body).unwrap();
        CacheableHttpResponse::new(inner)
    }
}
