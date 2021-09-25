#[derive(Debug, Clone, Default)]
pub struct CacheConfig {
    pub cache_key_prefix: Option<String>,
    pub ttl: Option<u32>,
    pub stale_ttl: Option<u32>,
    pub cache_version: Option<u32>,
}
