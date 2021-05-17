//! Module that implements transitions between states of the Hitbox finite state machine.
/// transition [show uml diagram](http://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/hit-box/hitbox/issue-14/hitbox/diagrams/only_cache.puml)
pub mod only_cache;
/// transition [show uml diagram](http://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/hit-box/hitbox/issue-14/hitbox/diagrams/stale.puml)
pub mod stale;
/// transition [show uml diagram](http://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/hit-box/hitbox/issue-14/hitbox/diagrams/upstream.puml)
pub mod upstream;
