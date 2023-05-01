use std::{collections::HashMap, fmt::Debug};

use chrono::Utc;
use hitbox::CachedValue;
use hitbox_backend::response2::CacheableResponse;
use http::{Response, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct HttpResponse<Body> {
    pub inner: Response<Body>,
}

impl<Body> HttpResponse<Body> {
    pub fn new(inner: Response<Body>) -> Self {
        HttpResponse { inner }
    }
}

impl<Body, E: Debug> From<Result<Response<Body>, E>> for HttpResponse<Body> {
    fn from(result: Result<Response<Body>, E>) -> Self {
        HttpResponse::new(result.unwrap())
    }
}

pub enum CachePolicy<Cached> {
    Cacheable(Cached),
    NonCacheable,
}

#[derive(Debug)]
pub enum CacheState<Cached> {
    Stale(Cached),
    Actual(Cached),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableHttpResponse<Body> {
    status: u16,
    version: String,
    headers: HashMap<String, String>,
    body: Body,
}

pub trait Cacheable: Sized {
    type Cached;

    fn cache_policy<'a>(&'a self) -> CachePolicy<CachedValue<Self::Cached>>
    where
        Self::Cached: From<&'a Self>,
    {
        if self.is_cacheable() {
            let cached = self.into();
            let cached_value = CachedValue::new(cached, Utc::now());
            CachePolicy::Cacheable(cached_value)
        } else {
            CachePolicy::NonCacheable
        }
    }

    fn from_cached(cached: CachedValue<Self::Cached>) -> CacheState<Self>
    where
        Self: From<Self::Cached>,
    {
        // TODO: check stale state
        CacheState::Actual(Self::from(cached.into_inner()))
    }

    fn is_cacheable(&self) -> bool;
}

impl<Body> CacheableResponse for HttpResponse<Body> {
    type Cached = SerializableHttpResponse<Body>;

    fn is_cacheable(&self) -> bool {
        self.inner.status() == StatusCode::OK
    }
}

impl<'a, Body: Clone> From<&'a HttpResponse<Body>> for SerializableHttpResponse<Body> {
    fn from(response: &'a HttpResponse<Body>) -> Self {
        let body = response.inner.body().clone();
        SerializableHttpResponse {
            status: response.inner.status().as_u16(),
            version: format!("{:?}", response.inner.version()),
            headers: response
                .inner
                .headers()
                .iter()
                .map(|(h, v)| (h.to_string(), v.to_str().unwrap().to_string()))
                .collect(),
            body,
        }
    }
}

impl<Body> From<SerializableHttpResponse<Body>> for HttpResponse<Body>
where
    Body: TryFrom<Body>,
{
    fn from(serializable: SerializableHttpResponse<Body>) -> Self {
        let body = match serializable.body.try_into() {
            Ok(res) => res,
            Err(_) => unimplemented!(),
        };
        let mut inner = Response::builder();
        for (key, value) in serializable.headers.into_iter() {
            inner = inner.header(key, value)
        }
        let inner = inner.status(200).body(body).unwrap();
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
}

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
