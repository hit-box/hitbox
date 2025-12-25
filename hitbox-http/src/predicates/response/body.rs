//! Response body matching predicates.
//!
//! Re-exports body predicates from the shared [`body`](crate::predicates::body) module.

pub use crate::predicates::body::{
    Body, BodyPredicate, JqExpression, JqExpression as JqFilter, JqOperation, Operation,
    PlainOperation,
};
