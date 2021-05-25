//! Cache settings declaration.

/// Cache setting status state.
#[derive(Debug, Clone)]
pub enum Status {
    /// Setting is enabled.
    Enabled,
    /// Setting is disabled.
    Disabled,
}

/// Describes all awailable cache settings.
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// Enable or disable cache at all.
    pub cache: Status,
    /// Enable or disable cache stale mechanics.
    pub stale: Status,
    /// Enable or disable cache lock mechanics.
    pub lock: Status,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InitialCacheSettings {
    Disabled,
    Enabled,
    Stale,
    Lock,
    StaleLock,
}

impl From<CacheSettings> for InitialCacheSettings {
    fn from(settings: CacheSettings) -> Self {
        match settings {
            CacheSettings {
                cache: Status::Disabled,
                ..
            } => InitialCacheSettings::Disabled,
            CacheSettings {
                cache: Status::Enabled,
                stale: Status::Disabled,
                lock: Status::Disabled,
            } => InitialCacheSettings::Enabled,
            CacheSettings {
                cache: Status::Enabled,
                stale: Status::Enabled,
                lock: Status::Disabled,
            } => InitialCacheSettings::Stale,
            CacheSettings {
                cache: Status::Enabled,
                stale: Status::Disabled,
                lock: Status::Enabled,
            } => InitialCacheSettings::Lock,
            CacheSettings {
                cache: Status::Enabled,
                stale: Status::Enabled,
                lock: Status::Enabled,
            } => InitialCacheSettings::StaleLock,
        }
    }
}
