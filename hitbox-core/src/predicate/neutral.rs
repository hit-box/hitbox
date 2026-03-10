//! Neutral predicate that always returns `Cacheable`.

use std::marker::PhantomData;

use async_trait::async_trait;

use super::{Predicate, PredicateResult};

/// A predicate that always returns `Cacheable`.
///
/// Useful as a starting point for predicate chains or as a no-op predicate.
///
/// # Type Parameters
///
/// * `S` - The subject type
/// * `Ctx` - The context type (defaults to `()`)
#[derive(Clone, Copy)]
pub struct Neutral<S, Ctx = ()> {
    #[allow(clippy::type_complexity)]
    _phantom: PhantomData<fn(S, Ctx) -> (S, Ctx)>,
}

impl<S, Ctx> std::fmt::Debug for Neutral<S, Ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Neutral").finish()
    }
}

impl<S, Ctx> Default for Neutral<S, Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, Ctx> Neutral<S, Ctx> {
    /// Creates a new neutral predicate.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<S, Ctx> Predicate for Neutral<S, Ctx>
where
    S: Send,
    Ctx: Default + Send,
{
    type Subject = S;
    type Context = Ctx;

    async fn check(
        &self,
        subject: Self::Subject,
        _ctx: &mut Self::Context,
    ) -> PredicateResult<Self::Subject> {
        PredicateResult::Cacheable(subject)
    }
}
