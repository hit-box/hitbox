//! Race write policy implementation.
//!
//! This policy writes to both L1 and L2 simultaneously and returns as soon as
//! the first write succeeds, handling the losing future based on the configured policy.

use async_trait::async_trait;
use futures::future::{Either, select};
use hitbox_core::{CacheKey, Offload};
use std::future::Future;

use super::CompositionWritePolicy;
use crate::BackendError;
use crate::composition::CompositionError;

/// Policy for handling the losing future in a race.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RaceLoserPolicy {
    /// Offload the losing future to background execution.
    /// This ensures the operation completes without blocking the response.
    #[default]
    Offload,
    /// Drop the losing future immediately.
    /// The operation may be cancelled mid-flight.
    Drop,
}

/// Race write policy: Write to both simultaneously, return on first success.
///
/// This strategy provides:
/// - Minimal write latency (return as soon as one succeeds)
/// - High availability (succeeds if either layer is available)
///
/// # Behavior
/// 1. Start both `write_l1(key)` and `write_l2(key)` in parallel using `select`
/// 2. When the first completes:
///    - If success: Return Ok immediately, handle losing future based on policy
///    - If failure: Wait for the second to complete
/// 3. If both fail: Return error with both failures
///
/// # Loser Policy
/// When one backend wins with a successful write, the losing future can be:
/// - `RaceLoserPolicy::Offload` (default): Spawned to background, completes without blocking
/// - `RaceLoserPolicy::Drop`: Dropped immediately, may cancel mid-operation
///
/// # Consistency Guarantee
/// If this operation returns `Ok(())`, **at least one** of L1 or L2 has been updated.
/// With `RaceLoserPolicy::Offload`, the other layer will eventually be updated (unless it fails).
/// With `RaceLoserPolicy::Drop`, the other layer may or may not be updated.
///
/// # Tradeoffs
/// - **Pros**: Lowest latency, high availability
/// - **Cons**: Only one layer guaranteed to be written at return time
///
/// # Use Cases
/// - Latency-critical write paths where one layer is sufficient
/// - Caches with background reconciliation
/// - Write-heavy workloads where L2 persistence is less critical
///
/// # Note
/// If you need both layers to be written, use [`super::OptimisticParallelWritePolicy`] instead,
/// which waits for both writes to complete.
#[derive(Debug, Clone, Copy, Default)]
pub struct RaceWritePolicy {
    /// Policy for handling the losing future.
    loser_policy: RaceLoserPolicy,
}

impl RaceWritePolicy {
    /// Create a new race write policy with default settings (offload losers).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the policy for handling losing futures.
    pub fn loser_policy(mut self, policy: RaceLoserPolicy) -> Self {
        self.loser_policy = policy;
        self
    }
}

#[async_trait]
impl CompositionWritePolicy for RaceWritePolicy {
    #[tracing::instrument(skip(self, key, write_l1, write_l2, offload), level = "trace")]
    async fn execute_with<F1, F2, Fut1, Fut2, O>(
        &self,
        key: CacheKey,
        write_l1: F1,
        write_l2: F2,
        offload: &O,
    ) -> Result<(), BackendError>
    where
        F1: FnOnce(CacheKey) -> Fut1 + Send,
        F2: FnOnce(CacheKey) -> Fut2 + Send,
        Fut1: Future<Output = Result<(), BackendError>> + Send + 'static,
        Fut2: Future<Output = Result<(), BackendError>> + Send + 'static,
        O: Offload<'static>,
    {
        // Box futures so we can move them to offload if needed
        let l1_fut = Box::pin(write_l1(key.clone()));
        let l2_fut = Box::pin(write_l2(key));

        // Race both futures
        match select(l1_fut, l2_fut).await {
            Either::Left((l1_result, l2_fut)) => {
                // L1 completed first
                match l1_result {
                    Ok(()) => {
                        // L1 succeeded - handle losing L2 future based on policy
                        tracing::trace!("L1 write succeeded (won race)");
                        match self.loser_policy {
                            RaceLoserPolicy::Offload => {
                                offload.register(
                                    hitbox_core::OffloadKey::auto("race_write_l2_loser"),
                                    async move {
                                        let _ = l2_fut.await;
                                    },
                                );
                            }
                            RaceLoserPolicy::Drop => {
                                drop(l2_fut);
                            }
                        }
                        Ok(())
                    }
                    Err(e1) => {
                        // L1 failed - must wait for L2
                        tracing::trace!("L1 write failed, waiting for L2");
                        match l2_fut.await {
                            Ok(()) => {
                                tracing::trace!("L2 write succeeded after L1 failure");
                                Ok(())
                            }
                            Err(e2) => {
                                tracing::error!(
                                    l1_error = ?e1,
                                    l2_error = ?e2,
                                    "Both L1 and L2 writes failed"
                                );
                                Err(BackendError::InternalError(Box::new(
                                    CompositionError::BothLayersFailed { l1: e1, l2: e2 },
                                )))
                            }
                        }
                    }
                }
            }
            Either::Right((l2_result, l1_fut)) => {
                // L2 completed first
                match l2_result {
                    Ok(()) => {
                        // L2 succeeded - handle losing L1 future based on policy
                        tracing::trace!("L2 write succeeded (won race)");
                        match self.loser_policy {
                            RaceLoserPolicy::Offload => {
                                offload.register(
                                    hitbox_core::OffloadKey::auto("race_write_l1_loser"),
                                    async move {
                                        let _ = l1_fut.await;
                                    },
                                );
                            }
                            RaceLoserPolicy::Drop => {
                                drop(l1_fut);
                            }
                        }
                        Ok(())
                    }
                    Err(e2) => {
                        // L2 failed - must wait for L1
                        tracing::trace!("L2 write failed, waiting for L1");
                        match l1_fut.await {
                            Ok(()) => {
                                tracing::trace!("L1 write succeeded after L2 failure");
                                Ok(())
                            }
                            Err(e1) => {
                                tracing::error!(
                                    l1_error = ?e1,
                                    l2_error = ?e2,
                                    "Both L1 and L2 writes failed"
                                );
                                Err(BackendError::InternalError(Box::new(
                                    CompositionError::BothLayersFailed { l1: e1, l2: e2 },
                                )))
                            }
                        }
                    }
                }
            }
        }
    }
}
