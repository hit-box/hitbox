//! Module that implements transitions between states of the Hitbox finite state machine.
/// transition [Transition diagram](http://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/hit-box/hitbox/master/documentation/transitions/only_cache.puml)
pub mod only_cache;
/// transition [Transition diagram](http://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/hit-box/hitbox/master/documentation/transitions/stale.puml)
pub mod stale;
/// transition [Transition diagram](http://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/hit-box/hitbox/master/documentation/transitions/upstream.puml)
pub mod upstream;
