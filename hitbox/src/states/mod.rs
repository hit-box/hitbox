//! The set of states of the Hitbox finite state machine.
//!
//! # Motivation
//! Different caching options are suitable for different [load profiles].
//! We devised Hitbox as a one-stop solution and therefore provide several different caching policies.
//! They are all described in [InitialCacheSettings].
//! The behavior of Hitbox depends on which caching policy option is selected, which means that
//! all Hitbox actions depend on the initial state ([InitialCacheSettings]).
//! To implement caching logic that depends on the initial state, we chose a finite state machine.
//! State machine transitions are defined by the [transition_groups] module and correspond
//! to the [InitialCacheSettings] options.
//!
//! States shouldn't depend on specific implementations of the cache backend and upstream,
//! and to interact with them, many states contain an `adapter` field.
//! The adapter provides the necessary operations such as getting the cache by key,
//! saving the cache, etc.
//!
//! # States
//! The states module is a set of states of the Hitbox finite state machine.
//!
//! [transition_groups]: ../transition_groups/index.html
//! [InitialCacheSettings]: ../enum.InitialCacheSettings.html
//! [load profiles]: ../

/// Defines whether the result returned from upstream will be cached.
pub mod cache_policy;
/// Defines the state of the data that was retrieved from the cache.
pub mod cache_polled;
/// The state that Hitbox enters after updating the cache.
pub mod cache_updated;
/// Final state of Hitbox.
pub mod finish;
/// Initial state of Hitbox.
pub mod initial;
/// Defines the state of the data that was retrieved from the upstream.
pub mod upstream_polled;
