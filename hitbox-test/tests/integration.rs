use cucumber::writer::{basic::Coloring, Verbosity};
use cucumber::{World, WriterExt};
use hitbox_test::core::HitboxWorld;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    // Use workspace root target directory
    // CARGO_MANIFEST_DIR points to hitbox-test, so we go up one level to workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("Failed to find workspace root");
    let target_dir = workspace_root.join("target");

    // Ensure target directory exists
    std::fs::create_dir_all(&target_dir).expect("Failed to create target directory");

    let junit_path = target_dir.join("cucumber-junit.xml");
    let file = std::fs::File::create(&junit_path).expect("Failed to create JUnit XML file");

    HitboxWorld::cucumber()
        .with_writer(
            // Basic provides console output and can be combined with JUnit
            cucumber::writer::Basic::new(std::io::stdout(), Coloring::Auto, Verbosity::Default)
                .summarized()
                .tee(cucumber::writer::JUnit::for_tee(file, 0))
                .normalized(),
        )
        .run("tests/features")
        .await;
}
