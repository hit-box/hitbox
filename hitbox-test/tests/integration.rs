use cucumber::World;
use hitbox_test::core::HitboxWorld;

#[tokio::main]
async fn main() {
    // Enable backtrace for ALL panics - this should show panic location
    std::env::set_var("RUST_BACKTRACE", "full");

    // Install panic hook that prints immediately before cucumber catches it
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("\n!!! ORIGINAL PANIC !!!");
        default_hook(panic_info);
        eprintln!("!!! END ORIGINAL PANIC !!!\n");
    }));

    HitboxWorld::run("tests/features").await;
}
