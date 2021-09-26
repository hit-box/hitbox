#[derive(Debug, Clone, Default)]
pub struct CacheConfig {
    pub key_prefix: Option<String>,
    pub ttl: Option<u32>,
    pub stale_ttl: Option<u32>,
    pub version: Option<u32>,
}
