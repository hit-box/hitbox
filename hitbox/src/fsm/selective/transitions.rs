//! Selective FSM transition types.
//!
//! Transition enums represent the possible outcomes from each state's `.transition()` method.
//! Each transition enum has an `.into_state()` method to convert to the outer `SelectiveState` enum.

#![allow(missing_docs)]

use futures::future::BoxFuture;
use hitbox_core::tag::CacheTag;
use hitbox_core::{CacheKey, PredicateResult};
use tracing::Span;

use super::states::{CheckPredicate, ExtractKey, Passthrough, SelectiveState};

// =============================================================================
// CheckPredicateTransition
// =============================================================================

/// Transitions from CheckPredicate state.
pub enum CheckPredicateTransition<'a, Req, UF> {
    /// Request matched — extract cache key and request tags from this config.
    ExtractKey {
        extract_future: BoxFuture<'a, (Req, CacheKey, Vec<CacheTag>)>,
        config_index: usize,
    },
    /// Request did not match — try the next enabled config.
    NextConfig {
        predicate_future: BoxFuture<'a, PredicateResult<Req>>,
        config_index: usize,
    },
    /// No more configs to try — pass through to upstream.
    Passthrough { upstream_future: UF },
}

impl<'a, Req, UF> CheckPredicateTransition<'a, Req, UF> {
    pub fn into_state<Inner>(self, parent: &Span) -> SelectiveState<'a, Inner, Req, UF> {
        match self {
            CheckPredicateTransition::ExtractKey {
                extract_future,
                config_index,
            } => SelectiveState::ExtractKey {
                extract_future,
                state: Some(ExtractKey::new(config_index, parent)),
            },
            CheckPredicateTransition::NextConfig {
                predicate_future,
                config_index,
            } => SelectiveState::CheckPredicate {
                predicate_future,
                state: Some(CheckPredicate::new(config_index, parent)),
            },
            CheckPredicateTransition::Passthrough { upstream_future } => {
                SelectiveState::Passthrough {
                    upstream_future,
                    state: Some(Passthrough::new(parent)),
                }
            }
        }
    }
}

// =============================================================================
// ExtractKeyTransition
// =============================================================================

/// Transitions from ExtractKey state.
pub enum ExtractKeyTransition<Inner> {
    /// Cache key extracted — delegate to CacheFuture.
    RunCacheFuture { inner: Inner },
}

impl<Inner> ExtractKeyTransition<Inner> {
    pub fn into_state<'a, Req, UF>(self) -> SelectiveState<'a, Inner, Req, UF> {
        match self {
            ExtractKeyTransition::RunCacheFuture { inner } => {
                SelectiveState::RunCacheFuture { inner }
            }
        }
    }
}
