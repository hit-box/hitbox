use std::{collections::HashMap, convert::Infallible, fmt::Debug};

use async_trait::async_trait;
use bytes::Buf;
use chrono::Utc;
use hitbox::CachedValue;
use hitbox_backend::response2::{CacheableResponse, CacheableWrapper};
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
impl CacheableWrapper for HttpResponse<hyper::Body> {
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
impl<D> CacheableWrapper for HttpResponse<UnsyncBoxBody<D, axum::Error>>
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
/*

impl<Body, E: Debug> From<Result<Response<Body>, E>> for HttpResponse<Body> {
    fn from(result: Result<Response<Body>, E>) -> Self {
        HttpResponse::new(result.unwrap())
    }
}

impl<Body, E: Debug> From<HttpResponse<Body>> for Result<Response<Body>, E> {
    fn from(value: HttpResponse<Body>) -> Self {
        Ok(value.inner)
    }
}

// impl<'a, Body> From<&'a HttpResponse<Body>> for SerializableHttpResponse {
//     fn from(response: &'a HttpResponse<Body>) -> Self {
//         let body = response.inner.body().clone();
//         SerializableHttpResponse {
//             status: response.inner.status().as_u16(),
//             version: format!("{:?}", response.inner.version()),
//             headers: response
//                 .inner
//                 .headers()
//                 .iter()
//                 .map(|(h, v)| (h.to_string(), v.to_str().unwrap().to_string()))
//                 .collect(),
//             // body,
//         }
//     }
// }

impl From<HttpResponse<hyper::Body>> for Response<hyper::Body> {
    fn from(value: HttpResponse<hyper::Body>) -> Self {
        value.inner
    }
}

impl From<SerializableHttpResponse> for HttpResponse<()> {
    fn from(serializable: SerializableHttpResponse) -> Self {
        // let body = match serializable.body.try_into() {
        //     Ok(res) => res,
        //     Err(_) => unimplemented!(),
        // };
        let mut inner = Response::builder();
        for (key, value) in serializable.headers.into_iter() {
            inner = inner.header(key, value)
        }
        let inner = inner.status(200).body(()).unwrap();
        Self { inner }
    }
}

impl From<SerializableHttpResponse> for HttpResponse<hyper::Body> {
    fn from(serializable: SerializableHttpResponse) -> Self {
        // let body = match serializable.body.try_into() {
        //     Ok(res) => res,
        //     Err(_) => unimplemented!(),
        // };
        let mut inner = Response::builder();
        for (key, value) in serializable.headers.into_iter() {
            inner = inner.header(key, value)
        }
        let inner = inner.status(200).body(hyper::Body::empty()).unwrap();
        Self { inner }
    }
}

#[cfg(test)]
mod test {
    use http::{Response, StatusCode};

    use super::HttpResponse;
    use hitbox_backend::response2::{CachePolicy, CacheState, CacheableResponse};

    #[test]
    fn test_transformation() {
        let resp = Response::builder()
            .status(StatusCode::OK)
            .header("X-Host", "localhost")
            .body("test")
            .unwrap();
        let cacheable = HttpResponse::new(resp);
        match cacheable.cache_policy() {
            CachePolicy::Cacheable(value) => {
                dbg!(&value);
                let response: CacheState<HttpResponse<&str>> = HttpResponse::from_cached(value);
                dbg!(&response);
            }
            _ => unimplemented!(),
        };

        let resp = Response::builder()
            .status(StatusCode::OK)
            .header("X-Host", "localhost")
            .body("Vec<u8>".as_bytes().to_vec())
            .unwrap();
        let cacheable = HttpResponse::new(resp);
        match cacheable.cache_policy() {
            CachePolicy::Cacheable(value) => {
                dbg!(&value);
                let response: CacheState<HttpResponse<Vec<u8>>> = HttpResponse::from_cached(value);
                dbg!(&response);
            }
            _ => unimplemented!(),
        };
    }
} */

// =====================================================
/*use std::{collections::HashMap, fmt::Debug};

use bytes::Bytes;
use chrono::Utc;
use hitbox::{CachePolicy, CachedValue};
use http::{Response, StatusCode, Version};
use serde::{Deserialize, Deserializer, Serialize};

pub struct CacheableResponse {
    // response: Response<Body>,
    ser: SerializableResponse,
}

unsafe impl Sync for CacheableResponse {}

impl CacheableResponse {
    pub fn from_response<'a, Body>(response: &'a Response<Body>) -> Self
    where
        Vec<u8>: From<&'a Body>,
    {
        let version = format!("{:?}", response.version());
        let status = response.status().as_u16();
        let headers = response
                    .headers()
                    .iter()
                    .map(|(h, v)| (h.to_string(), v.to_str().unwrap().to_string()))
                    .collect();
        let body = response.body().into();
        let ser = SerializableResponse {
            status,
            version,
            headers,
            body,
        };
        Self { ser }
    }

    fn to_serializable(&self) -> &SerializableResponse {
        &self.ser
    }

    // pub fn into_response(self) -> Response<Body> {
        // self.response
    // }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableResponse {
    status: u16,
    version: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl SerializableResponse {
    fn into_cacheable_response(self) -> CacheableResponse {
        unimplemented!()
    }

    pub fn into_cached_value(self) -> CachedValue<Self> {
        CachedValue { data: self, expired: Utc::now() }
    }
}

impl hitbox::CacheableResponse for CacheableResponse
{
    type Cached = SerializableResponse;

    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        dbg!("=========================");
        let c = self.to_serializable();
        dbg!(c);
        dbg!("=========================");
        // match self.response {
        // Ok(response) => CachePolicy::Cacheable(&self.into_serializable()),
        // Err(_) => CachePolicy::NonCacheable(()),
        // }
        // CachePolicy::NonCacheable(())
        CachePolicy::Cacheable(&c)
    }

    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        dbg!("++++++++++++++++++");
        CachePolicy::NonCacheable(self)
    }

    fn from_cached(cached: Self::Cached) -> Self {
        cached.into_cacheable_response()
    }
} */
