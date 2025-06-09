use cucumber::World;
use hitbox_test::core::HitboxWorld;

fn main() {
    futures::executor::block_on(HitboxWorld::run("tests/features/basic.feature"));
}
