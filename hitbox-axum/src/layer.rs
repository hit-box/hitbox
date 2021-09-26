use crate::config::CacheConfig;
use crate::service::CacheService;
use tower_layer::Layer;

#[derive(Debug, Default)]
pub struct CacheLayer {
    cache_config: CacheConfig,
}

impl CacheLayer {
    pub fn build() -> CacheLayerBuilder {
        CacheLayerBuilder::default()
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheService<S>;

    fn layer(&self, service: S) -> Self::Service {
        CacheService::new(service, self.cache_config.clone())
    }
}

#[derive(Debug, Default)]
pub struct CacheLayerBuilder {
    key_prefix: Option<String>,
    ttl: Option<u32>,
    stale_ttl: Option<u32>,
    version: Option<u32>,
    by_method: bool,
    by_path: bool,
    path_parser: Option<fn (String) -> String>,
    by_headers: Vec<String>,
    by_query: bool,
    by_body: bool,
}

impl CacheLayerBuilder {
    pub fn key_prefix(mut self, prefix: &str) -> CacheLayerBuilder {
        self.key_prefix = Some(prefix.to_string());
        self
    }
    pub fn ttl(mut self, ttl: u32) -> CacheLayerBuilder {
        self.ttl = Some(ttl);
        self
    }
    pub fn stale_ttl(mut self, stale_ttl: u32) -> CacheLayerBuilder {
        self.stale_ttl = Some(stale_ttl);
        self
    }
    pub fn version(mut self, version: u32) -> CacheLayerBuilder {
        self.version = Some(version);
        self
    }
    pub fn by_method(mut self) -> CacheLayerBuilder {
        self.by_method = true;
        self
    }
    pub fn by_path(mut self) -> CacheLayerBuilder {
        self.by_path = true;
        self
    }
    pub fn path_parser(mut self, parser: fn (String) -> String) -> CacheLayerBuilder {
        self.path_parser = Some(parser);
        self
    }
    pub fn by_header(mut self, header: &str) -> CacheLayerBuilder {
        self.by_headers.push(header.to_string());
        self
    }
    pub fn by_query(mut self) -> CacheLayerBuilder {
        self.by_query = true;
        self
    }
    pub fn by_body(mut self) -> CacheLayerBuilder {
        self.by_body = true;
        self
    }

    pub fn finish(self) -> CacheLayer {
        CacheLayer {
            cache_config: CacheConfig {
                key_prefix: self.key_prefix,
                ttl: self.ttl,
                stale_ttl: self.stale_ttl,
                version: self.version,
                by_method: self.by_method,
                by_path: self.by_path,
                path_parser: self.path_parser,
                by_headers: self.by_headers,
                by_query: self.by_query,
                by_body: self.by_body,
            },
        }
    }
}
