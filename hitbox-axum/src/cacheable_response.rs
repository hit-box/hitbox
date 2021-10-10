use axum::http::Response;
use hitbox::response::CacheableResponse as HitboxCacheableResponse;
use hitbox::CachePolicy;


pub struct CacheableResponse<Request>(Response<Request>);

impl<Request> HitboxCacheableResponse for CacheableResponse<Request> {
    type Cached = ();

    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        todo!()
    }

    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        todo!()
    }

    fn from_cached(cached: Self::Cached) -> Self {
        todo!()
    }
}