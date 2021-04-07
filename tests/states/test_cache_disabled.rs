use actix_cache::states::{CacheSettings, SettingState, InitialState, Message};

#[derive(Debug, PartialEq)]
pub struct Ping;

#[derive(Debug, PartialEq)]
pub struct Pong;

impl Message for Ping {
    type Result = Pong;
    fn handler(&self) -> Self::Result {
        Pong
    }
}

#[test]
fn test_cache_disabled() {
    let settings = CacheSettings {
        cache: SettingState::Disabled,
        stale: SettingState::Disabled,
        lock: SettingState::Disabled,
    };
    let message = Ping;
    let initial_state = InitialState::from((settings, message));
    let finish = initial_state.poll_upstream().finish();
    assert_eq!(finish.result, Pong)
}