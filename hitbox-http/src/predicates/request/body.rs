//! Request body matching predicates.
//!
//! Re-exports body predicates from the shared [`body`](crate::predicates::body) module.

pub use crate::predicates::body::{
    Body, BodyPredicate, JqExpression, JqOperation, Operation, PlainOperation,
};
