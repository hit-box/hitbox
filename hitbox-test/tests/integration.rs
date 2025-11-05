use cucumber::World;
use hitbox_test::core::HitboxWorld;

#[tokio::main]
async fn main() {
    HitboxWorld::run("tests/features").await;
}
