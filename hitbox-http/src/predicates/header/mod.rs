//! Header matching predicates.
//!
//! Provides [`Header`] predicate and [`Operation`] for matching HTTP headers
//! in both requests and responses.

mod operation;
mod predicate;

pub use operation::Operation;
pub use predicate::{HasHeaders, Header, HeaderPredicate};
