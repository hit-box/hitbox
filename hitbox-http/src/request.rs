use hitbox::Cacheable;
use http::Request;

pub struct CacheableRequest<Body>(Request<Body>);

impl<Body> Cacheable for CacheableRequest<Body> {
    fn cache_key(&self) -> Result<String, hitbox::CacheError> {
        Ok("cache-key".to_owned())
    }

    fn cache_key_prefix(&self) -> String {
        "my::".to_owned()
    }
}

impl<Body> From<Request<Body>> for CacheableRequest<Body> {
    fn from(request: Request<Body>) -> Self {
        Self(request)
    }
}
