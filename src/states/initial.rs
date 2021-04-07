use crate::states::cache_polled::CacheStatus;
use crate::states::{CachePolled, UpstreamPolled};
use crate::{CacheSettings, Database, InitialStateSettings, Message, Sender, SettingState};
use std::fmt::Debug;
use actix::Message;
use crate::settings::{InitialStateSettings, CacheSettings, SettingState};

#[derive(Debug)]
pub struct InitialState<M>
where
    M: Message,
{
    pub settings: InitialStateSettings,
    pub message: M,
}

impl<M, R> InitialState<M>
where
    M: Message<Result = R>,
    R: Debug,
{
    pub fn poll_cache(self) -> CachePolled<M, R> {
        let status = CacheStatus::Miss;
        println!("-> Poll cache: {:?}", status);
        CachePolled {
            cache_status: status,
            message: self.message,
        }
    }
    pub fn poll_upstream(self) -> UpstreamPolled<R> {
        println!("-> Poll upstream");
        let result = Database.send(&self.message);
        UpstreamPolled {
            upstream_result: result,
        }
    }
}

impl<M> From<(CacheSettings, M)> for InitialState<M>
where
    M: Message,
{
    fn from(settings: (CacheSettings, M)) -> Self {
        let (settings, message) = settings;
        let settings = match settings {
            CacheSettings {
                cache: SettingState::Disabled,
                ..
            } => InitialStateSettings::CacheDisabled,
            CacheSettings {
                cache: SettingState::Enabled,
                stale: SettingState::Disabled,
                lock: SettingState::Disabled,
            } => InitialStateSettings::CacheEnabled,
            CacheSettings {
                cache: SettingState::Enabled,
                stale: SettingState::Enabled,
                lock: SettingState::Disabled,
            } => InitialStateSettings::CacheStale,
            CacheSettings {
                cache: SettingState::Enabled,
                stale: SettingState::Disabled,
                lock: SettingState::Enabled,
            } => InitialStateSettings::CacheLock,
            CacheSettings {
                cache: SettingState::Enabled,
                stale: SettingState::Enabled,
                lock: SettingState::Enabled,
            } => InitialStateSettings::CacheStaleLock,
        };
        Self { settings, message }
    }
}
