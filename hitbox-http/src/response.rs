use std::{collections::HashMap, fmt::Debug};

use hitbox::CachePolicy;
use http::Response;
use serde::{Deserialize, Deserializer, Serialize};

pub struct CacheableResponse<Body> {
    response: Response<Body>,
    ser: SerializableResponse,
}

unsafe impl<Body> Sync for CacheableResponse<Body> {}

impl<Body> CacheableResponse<Body> {
    pub fn from_response(response: Response<Body>) -> Self {
        CacheableResponse {
            ser: SerializableResponse {
                method: response.status().to_string(),
                headers: response
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

    pub fn into_response(self) -> Response<Body> {
        self.response
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableResponse {
    method: String,
    headers: HashMap<String, String>,
}

impl<Body> From<CacheableResponse<Body>> for Response<Body> {
    fn from(value: CacheableResponse<Body>) -> Self {
        value.response
    }
}

impl<Body> From<Response<Body>> for CacheableResponse<Body> {
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
            response,
        }
    }
}

impl<Body> From<SerializableResponse> for CacheableResponse<Body> {
    fn from(value: SerializableResponse) -> Self {
        dbg!(value);
        unimplemented!()
    }
}

impl<Body> hitbox::CacheableResponse for CacheableResponse<Body> {
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
