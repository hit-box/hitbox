use hitbox::CachePolicy;
use http::Response;
use serde::Serialize;

pub struct CacheableResponse<Body>(Response<Body>);

impl<Body> From<Response<Body>> for CacheableResponse<Body> {
    fn from(response: Response<Body>) -> Self {
        CacheableResponse(response)
    }
}

impl<Body> hitbox::CacheableResponse for CacheableResponse<Body> 
where
    Body: Serialize,
{
    type Cached = Body;

    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        CachePolicy::NonCacheable(())
    }

    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        CachePolicy::NonCacheable(self)
    }

    fn from_cached(cached: Self::Cached) -> Self {
        let response = Response::builder()
            .body(cached).unwrap();
        response.into()
    }
}
