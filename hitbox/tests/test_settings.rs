// use fsm_cache::{CacheSettings, SettingState, InitialState, Message, InitialStateSettings};
// use actix::Message;
// use hitbox::settings::{CacheSettings, SettingState, InitialStateSettings};
//
// #[derive(Debug, PartialEq)]
// pub struct Ping;
//
// #[derive(Debug, PartialEq)]
// pub struct Pong;
//
//
// #[test]
// fn test_settings() {
//     let settings = CacheSettings {
//         cache: SettingState::Disabled,
//         stale: SettingState::Disabled,
//         lock: SettingState::Disabled,
//     };
//     let message = Ping;
//     let initial_state = InitialState::from((settings, message));
//     assert_eq!(initial_state.settings, InitialStateSettings::CacheDisabled);
//     assert_eq!(initial_state.message, Ping);
// }