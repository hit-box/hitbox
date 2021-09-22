use axum::http::Request;
use hitbox::cache::Cacheable;
use hitbox::CacheError;

pub struct CacheableRequest<T> {
    pub request: Request<T>,
}

impl<T> CacheableRequest<T> {
    pub fn into_inner(self) -> Request<T> {
        self.request
    }
}

impl<T> Cacheable for CacheableRequest<T> {
    fn cache_key(&self) -> Result<String, CacheError> {
        let path = self.request.uri().path();
        let method = self.request.method();
        let query = self.request.uri().query().unwrap_or_default();
        Ok(format!("{}:{}:{}", path, method, query))
    }

    fn cache_key_prefix(&self) -> String {
        todo!()
    }

    fn cache_ttl(&self) -> u32 {
        todo!()
    }

    fn cache_stale_ttl(&self) -> u32 {
        todo!()
    }

    fn cache_version(&self) -> u32 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::CacheableRequest;
    use axum::http::Request;
    use hitbox::cache::Cacheable;

    #[test]
    fn test_cache_key() {
        let request = Request::new(String::from("Hello world"));
        let wrapper = CacheableRequest { request };
        assert_eq!(wrapper.cache_key().unwrap(), String::from("/:GET:"))
    }
}
