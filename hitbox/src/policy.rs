#[derive(Default, Debug)]
pub struct EnabledCacheConfig {
    ttl: Option<u32>,
    stale_cache: Option<u32>,
}

#[derive(Debug)]
pub enum PolicyConfig {
    Enabled(EnabledCacheConfig),
    Disabled,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self::Enabled(Default::default())
    }
}
