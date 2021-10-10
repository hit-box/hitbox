use axum::http::Response;
use hitbox::response::CacheableResponse;
use hitbox::CachePolicy;
use serde::Serialize;


pub struct AxumCacheableResponse<T: Serialize>(pub Response<T>);

impl<T: Serialize> CacheableResponse for AxumCacheableResponse<T> {
    type Cached = T;

    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        CachePolicy::Cacheable(self.0.body())
    }

    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        let (parts, body) = self.0.into_parts();
        CachePolicy::Cacheable(body)
    }

    fn from_cached(cached: Self::Cached) -> Self {
        AxumCacheableResponse(Response::new(cached))
    }
}