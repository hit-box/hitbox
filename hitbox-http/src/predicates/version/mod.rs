//! HTTP version matching predicates.
//!
//! Provides [`HttpVersion`] predicate and [`Operation`] for matching
//! HTTP protocol versions (HTTP/1.0, HTTP/1.1, HTTP/2, HTTP/3).

mod operation;
mod predicate;

pub use operation::Operation;
pub use predicate::{HasVersion, HttpVersion, VersionPredicate};
