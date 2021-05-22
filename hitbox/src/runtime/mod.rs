//! Cache backend runtime agnostic interaction.
mod adapter;

pub use adapter::{AdapterResult, EvictionPolicy, RuntimeAdapter, TtlSettings};
