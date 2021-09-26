use axum::http::Request;
use hitbox::cache::Cacheable;
use hitbox::CacheError;

use crate::config::CacheConfig;

pub struct CacheableRequest<T> {
    pub request: Request<T>,
    pub cache_config: CacheConfig,
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
        let prefix = self.cache_key_prefix();
        Ok(format!("{}{}:{}:{}", prefix, path, method, query))
    }

    fn cache_key_prefix(&self) -> String {
        match &self.cache_config.key_prefix {
            Some(x) => format!("{}:", x),
            None => "".to_string(),
        }
    }

    fn cache_ttl(&self) -> u32 {
        match &self.cache_config.ttl {
            Some(x) => *x,
            None => 60,
        }
    }

    fn cache_stale_ttl(&self) -> u32 {
        match &self.cache_config.stale_ttl {
            Some(x) => *x,
            None => 60,
        }
    }

    fn cache_version(&self) -> u32 {
        match &self.cache_config.version {
            Some(x) => *x,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::CacheableRequest;
    use axum::http::Request;
    use hitbox::cache::Cacheable;

    use crate::config::CacheConfig;

    #[test]
    fn test_cache_key() {
        let request = Request::new(String::from("Hello world"));
        let cache_config = CacheConfig::default();
        let wrapper = CacheableRequest {
            request,
            cache_config,
        };
        assert_eq!(wrapper.cache_key().unwrap(), String::from("/:GET:"))
    }
}
