use cucumber::World;
use hitbox_test::core::HitboxWorld;

fn main() {
    futures::executor::block_on(HitboxWorld::run(
        "/home/singulared/sources/hitbox/hitbox/hitbox-test/tests/features/basic.feature",
    ));
}
