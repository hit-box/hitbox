use cucumber::writer::Stats as _;
use cucumber::writer::{Basic, JUnit};
use cucumber::writer::{Verbosity, basic::Coloring};
use cucumber::{World, WriterExt};
use hitbox_test::core::HitboxWorld;
use hitbox_test::fsm::FsmWorld;
use std::fs::{File, create_dir_all};
use std::io::stdout;
use std::path::PathBuf;

#[tokio::main]
pub async fn main() {
    // Use workspace root target directory
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("Failed to find workspace root");
    let target_dir = workspace_root.join("target");

    create_dir_all(&target_dir).expect("Failed to create target directory");

    // Run integration tests (predicates, extractors, logical)
    println!("\n========== Running Integration Tests ==========\n");
    let junit_path = target_dir.join("cucumber-integration-junit.xml");
    let file = File::create(&junit_path).expect("Failed to create JUnit XML file");

    let integration = HitboxWorld::cucumber()
        .max_concurrent_scenarios(None)
        .with_writer(
            Basic::new(stdout(), Coloring::Auto, Verbosity::Default)
                .summarized()
                .tee(JUnit::for_tee(file, 0))
                .normalized(),
        )
        .filter_run("tests/features", |feature, _rule, scenario| {
            // Skip FSM features (handled by FsmWorld) and failed-allowed scenarios
            !feature
                .path
                .as_ref()
                .is_some_and(|p| p.to_string_lossy().contains("/fsm/"))
                && !scenario.tags.iter().any(|tag| tag == "allow.failed")
        })
        .await;

    // Run FSM tests
    println!("\n========== Running FSM Tests ==========\n");
    let junit_path = target_dir.join("cucumber-fsm-junit.xml");
    let file = File::create(&junit_path).expect("Failed to create JUnit XML file");

    let fsm = FsmWorld::cucumber()
        .max_concurrent_scenarios(None)
        .with_writer(
            Basic::new(stdout(), Coloring::Auto, Verbosity::Default)
                .summarized()
                .tee(JUnit::for_tee(file, 0))
                .normalized(),
        )
        .filter_run("tests/features/fsm", |_feature, _rule, scenario| {
            !scenario.tags.iter().any(|tag| tag == "allow.failed")
        })
        .await;

    println!("\n========== All BDD Tests Complete ==========\n");

    if integration.execution_has_failed() || fsm.execution_has_failed() {
        panic!("BDD tests had failures");
    }
}
