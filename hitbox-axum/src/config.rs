#[derive(Debug, Clone, Default)]
pub struct CacheConfig {
    pub key_prefix: Option<String>,
    pub ttl: Option<u32>,
    pub stale_ttl: Option<u32>,
    pub version: Option<u32>,
    pub by_method: bool,
    pub by_path: bool,
    pub path_parser: Option<fn (String) -> String>,
    pub by_headers: Vec<String>,
    pub by_query: bool,
}
