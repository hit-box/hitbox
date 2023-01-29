use hitbox::Cacheable;
use http::{Request, Uri};

pub struct CacheableRequest<'a, Body>{
    request: &'a Request<Body>,
    uri: &'a Uri,
}

impl<'a, Body> CacheableRequest<'a, Body> {
    pub fn from_request(request: &'a Request<Body>) -> Self {
        Self {
            request,
            uri: request.uri(),
        }
    }
}

impl<'a, Body> Cacheable for CacheableRequest<'a, Body> {
    fn cache_key(&self) -> Result<String, hitbox::CacheError> {
        Ok(format!("{}::{}", self.request.method(), self.uri.path()))
    }

    fn cache_key_prefix(&self) -> String {
        format!("{}::", self.uri.scheme_str().unwrap_or("cache"))
    }
}

impl<'a, Body> From<&'a Request<Body>> for CacheableRequest<'a, Body> {
    fn from(request: &'a Request<Body>) -> Self {
        Self::from_request(request)
    }
}
