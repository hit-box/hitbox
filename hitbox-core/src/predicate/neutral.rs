//! Neutral predicate that always returns `Cacheable`.

use std::marker::PhantomData;

use async_trait::async_trait;

use crate::EvalContext;

use super::{Predicate, PredicateResult};

/// A predicate that always returns `Cacheable`.
///
/// Useful as a starting point for predicate chains or as a no-op predicate.
#[derive(Clone, Copy)]
pub struct Neutral<S> {
    _phantom: PhantomData<fn(S) -> S>,
}

impl<S> std::fmt::Debug for Neutral<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Neutral").finish()
    }
}

impl<S> Default for Neutral<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Neutral<S> {
    /// Creates a new neutral predicate.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<S> Predicate for Neutral<S>
where
    S: Send,
{
    type Subject = S;

    async fn check(
        &self,
        subject: Self::Subject,
        _ctx: &EvalContext,
    ) -> PredicateResult<Self::Subject> {
        PredicateResult::Cacheable(subject)
    }
}
