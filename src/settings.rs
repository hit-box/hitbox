pub enum SettingState {
    Enabled,
    Disabled,
}

pub struct CacheSettings {
    pub cache: SettingState,
    pub stale: SettingState,
    pub lock: SettingState,
}

#[derive(Debug, PartialEq)]
pub enum InitialStateSettings {
    CacheDisabled,
    CacheEnabled,
    CacheStale,
    CacheLock,
    CacheStaleLock,
}
