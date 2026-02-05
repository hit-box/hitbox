//! Race read policy implementation.
//!
//! This policy queries both L1 and L2 simultaneously and returns the first
//! successful hit, minimizing tail latency at the cost of increased backend load.

use async_trait::async_trait;
use futures::future::{Either, select};
use hitbox_core::{BoxContext, CacheContext, CacheKey, CacheValue, Offload};
use std::future::Future;

use super::{CompositionReadPolicy, ReadResult};
use crate::composition::CompositionLayer;

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

/// Race read policy: Query both L1 and L2 simultaneously, return first hit.
///
/// This strategy provides:
/// - Minimal tail latency by racing both backends
/// - Resilience to variable L1 performance
/// - Guaranteed fastest response time
///
/// # Behavior
/// 1. Start both `read_l1(key)` and `read_l2(key)` in parallel
/// 2. Whichever completes first:
///    - If hit (Ok(Some)): Return immediately with its context
///    - If miss/error: Wait for the second backend
/// 3. Aggregate results if neither hit first
///
/// # Loser Policy
/// When one backend wins with a cache hit, the losing future can be:
/// - `RaceLoserPolicy::Offload` (default): Spawned to background, completes without blocking
/// - `RaceLoserPolicy::Drop`: Dropped immediately, may cancel mid-operation
///
/// # Tradeoffs
/// - **Pros**: Best latency, especially for P99/P999
/// - **Cons**: 2x backend load (always queries both layers)
///
/// # Use Cases
/// - L1 has variable/unpredictable latency (remote cache)
/// - Tail latency is critical (user-facing APIs)
/// - Backend capacity allows double load
///
/// # Note
/// The closures passed to `execute_with` are responsible for any post-processing
/// like L1 population or envelope wrapping.
#[derive(Debug, Clone, Copy, Default)]
pub struct RaceReadPolicy {
    /// Policy for handling the losing future.
    loser_policy: RaceLoserPolicy,
}

impl RaceReadPolicy {
    /// Create a new race read policy with default settings (offload losers).
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
impl CompositionReadPolicy for RaceReadPolicy {
    #[tracing::instrument(skip(self, key, read_l1, read_l2, offload), level = "trace")]
    async fn execute_with<T, E, F1, F2, Fut1, Fut2, O>(
        &self,
        key: CacheKey,
        read_l1: F1,
        read_l2: F2,
        offload: &O,
    ) -> Result<ReadResult<T>, E>
    where
        T: Send + 'static,
        E: Send + std::fmt::Debug + 'static,
        F1: FnOnce(CacheKey) -> Fut1 + Send,
        F2: FnOnce(CacheKey) -> Fut2 + Send,
        Fut1: Future<Output = (Result<Option<CacheValue<T>>, E>, BoxContext)> + Send + 'static,
        Fut2: Future<Output = (Result<Option<CacheValue<T>>, E>, BoxContext)> + Send + 'static,
        O: Offload<'static>,
    {
        // Box futures so we can move them to offload if needed
        let l1_fut = Box::pin(read_l1(key.clone()));
        let l2_fut = Box::pin(read_l2(key));

        // Race both futures
        match select(l1_fut, l2_fut).await {
            Either::Left(((l1_result, l1_ctx), l2_fut)) => {
                // L1 completed first
                if let Ok(Some(value)) = l1_result {
                    tracing::trace!("L1 hit (won race)");
                    // Handle losing L2 future based on policy
                    match self.loser_policy {
                        RaceLoserPolicy::Offload => {
                            offload.register(
                                hitbox_core::OffloadKey::auto("race_read_l2_loser"),
                                async move {
                                    let _ = l2_fut.await;
                                },
                            );
                        }
                        RaceLoserPolicy::Drop => {
                            drop(l2_fut);
                        }
                    }
                    return Ok(ReadResult {
                        value: Some(value),
                        source: CompositionLayer::L1,
                        context: l1_ctx,
                    });
                }

                // L1 miss/error, wait for L2
                tracing::trace!("L1 completed first without hit, waiting for L2");
                let (l2_result, l2_ctx) = l2_fut.await;

                // Aggregate results - use context from whichever layer provided the data
                match (l1_result, l2_result) {
                    (Ok(Some(value)), _) => {
                        tracing::trace!("L1 hit");
                        Ok(ReadResult {
                            value: Some(value),
                            source: CompositionLayer::L1,
                            context: l1_ctx,
                        })
                    }
                    (_, Ok(Some(value))) => {
                        tracing::trace!("L2 hit");
                        Ok(ReadResult {
                            value: Some(value),
                            source: CompositionLayer::L2,
                            context: l2_ctx,
                        })
                    }
                    (Ok(None), Ok(None)) => {
                        tracing::trace!("Both layers miss");
                        Ok(ReadResult {
                            value: None,
                            source: CompositionLayer::L2,
                            context: CacheContext::default().boxed(),
                        })
                    }
                    (Err(e1), Err(e2)) => {
                        tracing::error!(l1_error = ?e1, l2_error = ?e2, "Both layers failed");
                        Err(e2)
                    }
                    (Ok(None), Err(e)) | (Err(e), Ok(None)) => {
                        tracing::warn!(error = ?e, "One layer failed, one missed");
                        Ok(ReadResult {
                            value: None,
                            source: CompositionLayer::L2,
                            context: CacheContext::default().boxed(),
                        })
                    }
                }
            }
            Either::Right(((l2_result, l2_ctx), l1_fut)) => {
                // L2 completed first
                if let Ok(Some(value)) = l2_result {
                    tracing::trace!("L2 hit (won race)");
                    // Handle losing L1 future based on policy
                    match self.loser_policy {
                        RaceLoserPolicy::Offload => {
                            offload.register(
                                hitbox_core::OffloadKey::auto("race_read_l1_loser"),
                                async move {
                                    let _ = l1_fut.await;
                                },
                            );
                        }
                        RaceLoserPolicy::Drop => {
                            drop(l1_fut);
                        }
                    }
                    return Ok(ReadResult {
                        value: Some(value),
                        source: CompositionLayer::L2,
                        context: l2_ctx,
                    });
                }

                // L2 miss/error, wait for L1
                tracing::trace!("L2 completed first without hit, waiting for L1");
                let (l1_result, l1_ctx) = l1_fut.await;

                // Aggregate results - use context from whichever layer provided the data
                match (l1_result, l2_result) {
                    (Ok(Some(value)), _) => {
                        tracing::trace!("L1 hit");
                        Ok(ReadResult {
                            value: Some(value),
                            source: CompositionLayer::L1,
                            context: l1_ctx,
                        })
                    }
                    (_, Ok(Some(value))) => {
                        tracing::trace!("L2 hit");
                        Ok(ReadResult {
                            value: Some(value),
                            source: CompositionLayer::L2,
                            context: l2_ctx,
                        })
                    }
                    (Ok(None), Ok(None)) => {
                        tracing::trace!("Both layers miss");
                        Ok(ReadResult {
                            value: None,
                            source: CompositionLayer::L2,
                            context: CacheContext::default().boxed(),
                        })
                    }
                    (Err(e1), Err(e2)) => {
                        tracing::error!(l1_error = ?e1, l2_error = ?e2, "Both layers failed");
                        Err(e2)
                    }
                    (Ok(None), Err(e)) | (Err(e), Ok(None)) => {
                        tracing::warn!(error = ?e, "One layer failed, one missed");
                        Ok(ReadResult {
                            value: None,
                            source: CompositionLayer::L2,
                            context: CacheContext::default().boxed(),
                        })
                    }
                }
            }
        }
    }
}
