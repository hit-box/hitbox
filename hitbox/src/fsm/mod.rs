//! Finite State Machine for cache orchestration.
//!
//! Coordinates the entire cache lifecycle: checking predicates, looking up cache,
//! calling upstream on miss, applying stale policies, and updating cache with responses.

mod future;
mod selective;
mod states;
pub mod transitions;

pub use future::CacheFuture;
pub use selective::SelectiveCacheFuture;
pub use states::{PollCacheFuture, State, UpdateCache};
