use cucumber::{gherkin::Step, given, World};
use hitbox_test::Predicates;

// This runs before everything else, so you can setup things here.
fn main() {
    // You may choose any executor you like (`tokio`, `async-std`, etc.).
    // You may even have an `async` main, it doesn't matter. The point is that
    // Cucumber is composable. :)
    futures::executor::block_on(HitboxWorld::run("tests/features/basic.feature"));
}

#[derive(Debug, Default, World)]
pub struct HitboxWorld {
    predicates: Predicates,
}

#[given(regex = r"^hitbox with\s+(policy (.*))$")]
fn hitbox(_world: &mut HitboxWorld, step: &Step, policy: String) {
    dbg!(&step.docstring);
    dbg!(policy);
}

#[given(expr = "request predicate {word}")]
fn request_predicate(world: &mut HitboxWorld, step: &Step, predicate: String) {
    world.predicates.request.push(predicate);
    dbg!(&world);
}
