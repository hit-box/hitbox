use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EnabledCacheConfig {
    ttl: Option<u32>,
    stale: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyConfig {
    Enabled(EnabledCacheConfig),
    Disabled,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self::Enabled(EnabledCacheConfig {
            ttl: Some(5),
            stale: None,
        })
    }
}
