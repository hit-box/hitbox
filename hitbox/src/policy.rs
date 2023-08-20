#[derive(Default, Debug)]
pub struct EnabledCacheConfig {
    ttl: Option<std::time::Duration>,
    stale_cache: Option<std::time::Duration>,
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
