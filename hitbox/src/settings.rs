#[derive(Debug, Clone)]
pub enum Status {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct CacheSettings {
    pub cache: Status,
    pub stale: Status,
    pub lock: Status,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InitialCacheSettings {
    CacheDisabled,
    CacheEnabled,
    CacheStale,
    CacheLock,
    CacheStaleLock,
}

impl From<CacheSettings> for InitialCacheSettings {
    fn from(settings: CacheSettings) -> Self {
        match settings {
            CacheSettings {
                cache: Status::Disabled,
                ..
            } => InitialCacheSettings::CacheDisabled,
            CacheSettings {
                cache: Status::Enabled,
                stale: Status::Disabled,
                lock: Status::Disabled,
            } => InitialCacheSettings::CacheEnabled,
            CacheSettings {
                cache: Status::Enabled,
                stale: Status::Enabled,
                lock: Status::Disabled,
            } => InitialCacheSettings::CacheStale,
            CacheSettings {
                cache: Status::Enabled,
                stale: Status::Disabled,
                lock: Status::Enabled,
            } => InitialCacheSettings::CacheLock,
            CacheSettings {
                cache: Status::Enabled,
                stale: Status::Enabled,
                lock: Status::Enabled,
            } => InitialCacheSettings::CacheStaleLock,
        }
    }
}
