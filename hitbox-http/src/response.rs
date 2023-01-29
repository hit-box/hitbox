use std::{collections::HashMap, fmt::Debug};

use hitbox::CachePolicy;
use http::Response;
use serde::{Deserialize, Deserializer, Serialize};

pub struct CacheableResponse<Body, E> {
    response: Result<Response<Body>, E>,
    ser: SerializableResponse,
}

unsafe impl<Body, E> Sync for CacheableResponse<Body, E> {}

impl<Body, E: Debug> CacheableResponse<Body, E> {
    pub fn from_response(response: Result<Response<Body>, E>) -> Self {
        CacheableResponse {
            ser: SerializableResponse {
                method: response.as_ref().unwrap().status().to_string(),
                headers: response
                    .as_ref()
                    .unwrap()
                    .headers()
                    .iter()
                    .map(|(h, v)| (h.to_string(), v.to_str().unwrap().to_string()))
                    .collect(),
            },
            response,
        }
    }

    fn to_serializible(&self) -> &SerializableResponse {
        &self.ser
    }

    pub fn into_response(self) -> Result<Response<Body>, E> {
        self.response
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableResponse {
    method: String,
    headers: HashMap<String, String>,
}

impl<Body, E> From<CacheableResponse<Body, E>> for Result<Response<Body>, E> {
    fn from(value: CacheableResponse<Body, E>) -> Self {
        value.response
    }
}

impl<Body, E> From<Response<Body>> for CacheableResponse<Body, E> {
    fn from(response: Response<Body>) -> Self {
        CacheableResponse {
            ser: SerializableResponse {
                method: response.status().to_string(),
                headers: response
                    .headers()
                    .iter()
                    .map(|(h, v)| (h.to_string(), v.to_str().unwrap().to_string()))
                    .collect(),
            },
            response: Ok(response),
        }
    }
}

impl<Body, E> From<SerializableResponse> for CacheableResponse<Body, E> {
    fn from(value: SerializableResponse) -> Self {
        dbg!(value);
        unimplemented!()
    }
}

impl<Body, E: Debug> hitbox::CacheableResponse for CacheableResponse<Body, E> {
    type Cached = SerializableResponse;

    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        dbg!("=========================");
        let c = self.to_serializible();
        dbg!(c);
        dbg!("=========================");
        // match self.response {
        // Ok(response) => CachePolicy::Cacheable(&self.into_serializible()),
        // Err(_) => CachePolicy::NonCacheable(()),
        // }
        // CachePolicy::NonCacheable(())
        CachePolicy::Cacheable(c)
    }

    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        dbg!("++++++++++++++++++");
        CachePolicy::NonCacheable(self)
    }

    fn from_cached(cached: Self::Cached) -> Self {
        cached.into()
    }
}
