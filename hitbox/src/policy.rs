use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EnabledCacheConfig {
    ttl: Option<u32>,
    stale_cache: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyConfig {
    Enabled(EnabledCacheConfig),
    Disabled,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self::Enabled(Default::default())
    }
}
