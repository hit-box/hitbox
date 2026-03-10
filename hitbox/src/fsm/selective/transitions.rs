//! Selective FSM transition types.
//!
//! Transition enums represent the possible outcomes from each state's `.transition()` method.
//! Each transition enum has an `.into_state()` method to convert to the outer `SelectiveState` enum.

#![allow(missing_docs)]

use futures::future::BoxFuture;
use hitbox_core::{KeyParts, PredicateResult};
use tracing::Span;

use super::states::{CheckPredicate, ExtractKey, Passthrough, SelectiveState};

// =============================================================================
// CheckPredicateTransition
// =============================================================================

/// Transitions from CheckPredicate state.
pub enum CheckPredicateTransition<'a, Req, UF> {
    /// Request matched — extract cache key from this config.
    ExtractKey {
        extract_future: BoxFuture<'a, KeyParts<Req>>,
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

impl<Req, UF> std::fmt::Debug for CheckPredicateTransition<'_, Req, UF> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExtractKey { config_index, .. } => f
                .debug_struct("CheckPredicateTransition::ExtractKey")
                .field("config_index", config_index)
                .finish(),
            Self::NextConfig { config_index, .. } => f
                .debug_struct("CheckPredicateTransition::NextConfig")
                .field("config_index", config_index)
                .finish(),
            Self::Passthrough { .. } => f.write_str("CheckPredicateTransition::Passthrough"),
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

impl<Inner> std::fmt::Debug for ExtractKeyTransition<Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RunCacheFuture { .. } => f.write_str("ExtractKeyTransition::RunCacheFuture"),
        }
    }
}
