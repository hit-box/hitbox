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
        let mut key_parts: Vec<String> = Vec::new();

        let prefix = self.cache_key_prefix();
        if !prefix.is_empty() {
            key_parts.push(prefix)
        }

        if self.cache_config.by_method {
            key_parts.push(self.request.method().to_string())
        }

        if self.cache_config.by_path {
            let path = self.request.uri().path().to_string();
            match self.cache_config.path_parser {
                Some(parser) => key_parts.push(parser(path)),
                None => key_parts.push(path),
            }
        }

        let r_headers = self.request.headers();
        for header in &self.cache_config.by_headers {
            if let Some(v) = r_headers.get(header) { key_parts.push(v.to_str().unwrap_or_default().to_string()) }
        }

        if self.cache_config.by_query {
            if let Some(query) = self.request.uri().query() { key_parts.push(query.to_string()) }
        }

        Ok(key_parts.join(":"))
    }

    fn cache_key_prefix(&self) -> String {
        match &self.cache_config.key_prefix {
            Some(x) => x.to_string(),
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
        let request = Request::builder()
            .uri(String::from("/v1/seaweed"))
            .header("HEADER_FIRST", "and")
            .header("HEADER_SECOND", "will be happy")
            .body(String::from("body")).unwrap();
        let cache_config = CacheConfig{
            key_prefix: Some("ferris will".to_string()),
            ttl: None,
            stale_ttl: None,
            version: None,
            by_method: true,
            by_path: true,
            path_parser: Some(| path: String | -> String { path.trim_start_matches("/v1").to_string() }),
            by_headers: vec![String::from("HEADER_FIRST"), String::from("HEADER_SECOND")],
            by_query: true,
        };
        let wrapper = CacheableRequest {
            request,
            cache_config,
        };
        assert_eq!(wrapper.cache_key().unwrap(), String::from("ferris will:GET:/seaweed:and:will be happy"))
    }
}
