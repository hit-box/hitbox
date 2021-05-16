//! The set of states of the Hitbox finite state machine.
//!
//! # Motivation
//! Different caching options are suitable for different [load profiles].
//! We devised Hitbox as a one-stop solution and therefore provide several different caching policies.
//! They are all described in [InitialCacheSettings].
//! The behavior of Hitbox depends on which caching policy option is selected, which means that
//! all Hitbox actions depend on the initial state ([InitialCacheSettings]).
//! To implement caching logic that depends on and stores the initial state, we chose a finite state machine.
//! State machine transitions are defined by the [transition_groups] module and correspond
//! to the [InitialCacheSettings] options.
//!
//! # States
//! The states module is a set of states of the Hitbox finite state machine.
//! * [cache_policy] Defines whether the result returned from upstream will be cached.
//! * [cache_polled] Defines the state of the data that was retrieved from the cache.
//! * [cache_updated] The state that Hitbox enters after updating the cache.
//! * [finish] Final state of Hitbox.
//! * [initial] Initial state of Hitbox.
//! * [upstream_polled] Defines the state of the data that was retrieved from the upstream.
//!
//! [transition_groups]: ../transition_groups/index.html
//! [InitialCacheSettings]: ../enum.InitialCacheSettings.html
//! [load profiles]: ../

pub mod cache_policy;
pub mod cache_polled;
pub mod cache_updated;
pub mod finish;
pub mod initial;
pub mod upstream_polled;
