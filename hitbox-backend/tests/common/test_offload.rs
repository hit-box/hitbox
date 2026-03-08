//! Simple test offload implementation using tokio::spawn.

use std::future::Future;

use hitbox_core::Offload;
use smol_str::SmolStr;

/// Simple offload manager for tests that spawns tasks with tokio::spawn.
#[derive(Clone, Debug, Default)]
pub struct TestOffloadManager;

impl Offload<'static> for TestOffloadManager {
    #[allow(deprecated)]
    fn spawn<F>(&self, _kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future);
    }
}
