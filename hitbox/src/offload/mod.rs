//! Offload manager for background task execution.
//!
//! This module provides infrastructure for offloading tasks to background execution,
//! primarily used for Stale-While-Revalidate (SWR) cache pattern where stale data
//! is returned immediately while cache revalidation happens asynchronously.
//!
//! # Example
//!
//! ```ignore
//! use hitbox::offload::{OffloadManager, OffloadConfig};
//!
//! let config = OffloadConfig::default();
//! let manager = OffloadManager::new(config);
//!
//! // Spawn a background task
//! manager.spawn(async {
//!     // Revalidation logic here
//! });
//! ```

mod manager;
mod policy;

#[allow(deprecated)]
pub use manager::{OffloadHandle, OffloadKey, OffloadManager};
pub use policy::{OffloadConfig, OffloadConfigBuilder, TimeoutPolicy};
pub use smol_str::SmolStr;
