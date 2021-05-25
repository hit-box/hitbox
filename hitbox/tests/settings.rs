// use fsm_cache::{CacheSettings, SettingState, Initial, Message, InitialStateSettings};
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
//     let initial = Initial::from((settings, message));
//     assert_eq!(initial.settings, InitialStateSettings::CacheDisabled);
//     assert_eq!(initial.message, Ping);
// }
