use std::{collections::HashMap, convert::Infallible, fmt::Debug};

use async_trait::async_trait;
use bytes::Buf;
use chrono::Utc;
use hitbox::CachedValue;
use hitbox_backend::{CacheableResponse, CacheableResponseWrapper};
use http::{Response, StatusCode};
use http_body::{combinators::UnsyncBoxBody, Body, Empty, Full};
use hyper::body::to_bytes;
use serde::{Deserialize, Serialize};
use serde_bytes::Bytes;

#[derive(Debug)]
pub struct HttpResponse<Body> {
    pub inner: Response<Body>,
}

impl<Body> HttpResponse<Body> {
    pub fn new(inner: Response<Body>) -> Self {
        HttpResponse { inner }
    }
}

#[async_trait]
impl CacheableResponseWrapper for HttpResponse<hyper::Body> {
    type Source = Result<Response<hyper::Body>, Infallible>;
    type Serializable = SerializableHttpResponse;
    type Error = hyper::Error;

    fn from_source(source: Self::Source) -> Self {
        HttpResponse::new(source.unwrap())
    }

    fn into_source(self) -> Self::Source {
        Ok(self.inner)
    }

    fn from_serializable(serializable: Self::Serializable) -> Self {
        let mut inner = Response::builder();
        for (key, value) in serializable.headers.into_iter() {
            inner = inner.header(key, value)
        }
        let body = serializable.body.into();
        let inner = inner.status(serializable.status).body(body).unwrap();
        Self { inner }
    }

    async fn into_serializable(self) -> Result<Self::Serializable, Self::Error> {
        let source = self.inner;
        Ok(SerializableHttpResponse {
            status: source.status().as_u16(),
            version: format!("{:?}", source.version()),
            headers: source
                .headers()
                .iter()
                .map(|(h, v)| (h.to_string(), v.to_str().unwrap().to_string()))
                .collect(),
            body: to_bytes(source.into_body()).await?.to_vec(),
        })
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

impl CacheableResponse for HttpResponse<hyper::Body> {
    type Cached = SerializableHttpResponse;

    fn is_cacheable(&self) -> bool {
        self.inner.status() == StatusCode::OK
    }
}

impl<D> CacheableResponse for HttpResponse<UnsyncBoxBody<D, axum::Error>>
where
    D: Buf + Send + From<Vec<u8>> + 'static,
{
    type Cached = SerializableHttpResponse;

    fn is_cacheable(&self) -> bool {
        self.inner.status() == StatusCode::OK
    }
}

#[async_trait]
impl<D> CacheableResponseWrapper for HttpResponse<UnsyncBoxBody<D, axum::Error>>
where
    D: From<Vec<u8>> + Buf + Send + 'static,
{
    type Source = Result<Response<UnsyncBoxBody<D, axum::Error>>, Infallible>;
    type Serializable = SerializableHttpResponse;
    type Error = hyper::Error;

    fn from_source(source: Self::Source) -> Self {
        HttpResponse::new(source.unwrap())
    }

    fn into_source(self) -> Self::Source {
        Ok(self.inner)
    }

    fn from_serializable(serializable: Self::Serializable) -> Self {
        let mut inner = Response::builder();
        for (key, value) in serializable.headers.into_iter() {
            inner = inner.header(key, value)
        }
        let body = UnsyncBoxBody::new(
            Full::from(serializable.body).map_err(|error| axum::Error::new(error)),
        );
        let inner = inner.status(serializable.status).body(body).unwrap();
        Self { inner }
    }

    async fn into_serializable(self) -> Result<Self::Serializable, Self::Error> {
        let source = self.inner;
        let status = source.status().as_u16();
        let headers = source
            .headers()
            .iter()
            .map(|(h, v)| (h.to_string(), v.to_str().unwrap().to_string()))
            .collect();
        Ok(SerializableHttpResponse {
            status,
            version: format!("{:?}", source.version()),
            headers,
            body: to_bytes(source.into_body()).await.unwrap().to_vec(),
        })
    }
}
