use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for enabled cache
#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq, JsonSchema)]
pub struct EnabledCacheConfig {
    pub ttl: Option<u32>,
    pub stale: Option<u32>,
}

/// Cache policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, JsonSchema)]
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

impl PolicyConfig {
    /// Convert from hitbox::policy::PolicyConfig
    pub fn from_policy(config: hitbox::policy::PolicyConfig) -> Self {
        match config {
            hitbox::policy::PolicyConfig::Enabled(enabled) => {
                PolicyConfig::Enabled(EnabledCacheConfig {
                    ttl: enabled.ttl,
                    stale: enabled.stale,
                })
            }
            hitbox::policy::PolicyConfig::Disabled => PolicyConfig::Disabled,
        }
    }

    /// Convert to hitbox::policy::PolicyConfig
    pub fn into_policy(self) -> hitbox::policy::PolicyConfig {
        match self {
            PolicyConfig::Enabled(enabled) => {
                hitbox::policy::PolicyConfig::Enabled(hitbox::policy::EnabledCacheConfig {
                    ttl: enabled.ttl,
                    stale: enabled.stale,
                })
            }
            PolicyConfig::Disabled => hitbox::policy::PolicyConfig::Disabled,
        }
    }
}
